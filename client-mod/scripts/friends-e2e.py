#!/usr/bin/env python3
"""Drive two real game windows; inspect the local dev server to verify their actions."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import urllib.parse
import urllib.request

base, version, output = sys.argv[1:4]
if urllib.parse.urlsplit(base).hostname not in ("127.0.0.1", "localhost", "::1"):
    raise SystemExit("This test requires an isolated loopback dev server.")
output = Path(output)
output.mkdir(parents=True, exist_ok=True)


def api(method, path, token=None, body=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = "Bearer " + token
    data = None if body is None else json.dumps(body).encode()
    with urllib.request.urlopen(urllib.request.Request(base + path, data=data, headers=headers, method=method), timeout=20) as reply:
        raw = reply.read()
        return json.loads(raw) if raw else None


def login(name):
    start = api("POST", "/v1/auth/start", body={"name": name})
    return api("POST", "/v1/auth/finish", body={"name": name, "serverId": start["serverId"]})


def wait(check, message, timeout=90):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        value = check()
        if value:
            return value
        time.sleep(.2)
    raise AssertionError(message)


def xdo(*args):
    return subprocess.check_output(["xdotool", *map(str, args)], text=True).strip()


def window(name):
    for proc in Path("/proc").iterdir():
        if not proc.name.isdigit():
            continue
        try:
            env = (proc / "environ").read_bytes().split(b"\0")
            cmd = (proc / "cmdline").read_bytes().split(b"\0")
            if ("E2E_PLAYER=" + name).encode() not in env or Path(os.fsdecode(cmd[0])).name != "java":
                continue
            if os.environ.get("CAIRN_HOME") and ("CAIRN_HOME=" + os.environ["CAIRN_HOME"]).encode() not in env:
                continue
            found = subprocess.run(["xdotool", "search", "--pid", proc.name], text=True, capture_output=True)
            for wid in found.stdout.split():
                geometry = xdo("getwindowgeometry", "--shell", wid)
                if "WIDTH=1280\n" in geometry and "HEIGHT=720\n" in geometry:
                    return wid
        except (OSError, subprocess.CalledProcessError):
            continue
    if version.startswith("1.8"):
        found = subprocess.run(["xdotool", "search", "--name", "Minecraft"], text=True, capture_output=True)
        windows = sorted(found.stdout.split(), key=int)
        index = 0 if name == "CairnAlice" else 1
        if len(windows) > index:
            return windows[index]
    return None


def focus(wid):
    xdo("windowraise", wid, "windowfocus", "--sync", wid)
    time.sleep(.15)


def click(wid, x, y):
    focus(wid)
    xdo("mousemove", "--sync", "--window", wid, x, y)
    time.sleep(.2)
    xdo("mousedown", 1)
    time.sleep(.15)
    xdo("mouseup", 1)
    time.sleep(.3)


def key(wid, key):
    focus(wid)
    xdo("key", key)
    time.sleep(.5)


def capture(wid, name):
    focus(wid)
    time.sleep(.3)
    subprocess.run(["import", "-window", wid, str(output / (name + ".png"))], check=True)


alice_window = wait(lambda: window("CairnAlice"), "Alice's game window did not open", 300)
bob_window = wait(lambda: window("CairnBob"), "Bob's game window did not open", 300)
time.sleep(10)
alice, bob = login("CairnAlice"), login("CairnBob")
a, b = alice["token"], bob["token"]


def friends(token):
    return api("GET", "/v1/friends", token)


def bob_from_alice():
    return next((f for f in friends(a)["friends"] if f["uuid"] == bob["uuid"]), None)


# E2E_FRIENDS fixes GUI scale at 2, so these are game pixels, independent of the desktop size.
key(alice_window, "Shift_R")
capture(alice_window, "initial-" + version)
click(alice_window, 150, 195)
xdo("type", "--clearmodifiers", "--delay", 60, "CairnBob")
click(alice_window, 1000, 195)
capture(alice_window, "sent-" + version)
wait(lambda: friends(b)["incoming"], "The in-game Add button did not send a request")
capture(bob_window, "toast-" + version)
key(bob_window, "Shift_R")
click(bob_window, 910, 305)
wait(bob_from_alice, "The in-game Accept button did not create the friendship")
wait(lambda: (bob_from_alice().get("activity") or {}).get("server"), "Bob must be connected to the local Minecraft server")
assert bob_from_alice()["activity"]["version"] == version
capture(alice_window, "friends-" + version)
capture(alice_window, "toast-accepted-" + version)

click(alice_window, 950, 305)
wait(lambda: next((f for f in friends(b)["friends"] if f["uuid"] == alice["uuid"] and f.get("activity") and f["activity"]["mode"] == "multiplayer"), None), "Join did not connect Alice to Bob's server")

click(bob_window, 990, 140)
wait(lambda: api("GET", "/v1/me/social", b)["dnd"], "DND did not reach the server")
capture(bob_window, "dnd-" + version)
click(bob_window, 1150, 195)
click(bob_window, 230, 255)
wait(lambda: bob_from_alice()["activity"] is not None and bob_from_alice()["activity"]["server"] is None, "Hide server leaked the address")
click(bob_window, 600, 255)
wait(lambda: bob_from_alice()["status"] == "offline" and bob_from_alice()["activity"] is None, "Appear offline leaked activity")

other = login("CairnToast")
api("POST", "/v1/friends/requests", other["token"], {"target": "CairnBob"})
time.sleep(2)
capture(bob_window, "dnd-muted-request-" + version)
click(bob_window, 1150, 195)
# The incoming request occupies two rows, so Alice's existing friend row is lower.
click(bob_window, 1190, 410)
wait(lambda: bob_from_alice() is None, "Blocking did not remove the friendship")
assert any(p["uuid"] == alice["uuid"] for p in friends(b)["blocked"])
assert all(e.get("from", {}).get("uuid") != bob["uuid"] for e in api("GET", "/v1/events?after=0", a)["events"])
print(json.dumps({"version": version, "request": "passed", "accept": "passed", "presence": "passed", "join": "passed", "hideServer": "passed", "appearOffline": "passed", "block": "passed", "dnd": "screenshots captured; inspect toast absence"}, indent=2))
