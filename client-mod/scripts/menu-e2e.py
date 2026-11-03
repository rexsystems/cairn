#!/usr/bin/env python3
"""Exercise the native menu adapters in isolated offline Minecraft processes."""
import json
import os
from pathlib import Path
import signal
import subprocess
import time

root = Path(__file__).resolve().parents[2]
work = Path(os.environ["CAIRN_MENU_SMOKE_DIR"])
output = work / "menu-captures"
output.mkdir(parents=True, exist_ok=True)
versions = os.environ.get("CAIRN_MENU_VERSIONS", "26.3,1.21.11,1.21.1,1.20.4,1.20.1,1.16.5,1.8.9,1.8.8,1.8").split(",")


def xdo(*args):
    return subprocess.check_output(["xdotool", *map(str, args)], text=True).strip()


def window():
    found = subprocess.run(["xdotool", "search", "--class", "."], capture_output=True, text=True)
    for wid in found.stdout.split():
        try:
            geometry = xdo("getwindowgeometry", "--shell", wid)
            if "WIDTH=1920\n" in geometry and "HEIGHT=1080\n" in geometry:
                return wid
        except subprocess.CalledProcessError:
            pass
    return None


def click(wid, x, y):
    xdo("windowraise", wid, "windowfocus", "--sync", wid)
    xdo("mousemove", "--window", wid, x, y)
    time.sleep(.15)
    xdo("mousedown", 1)
    time.sleep(.1)
    xdo("mouseup", 1)
    time.sleep(.3)


def capture(wid, name):
    time.sleep(.3)
    subprocess.run(["import", "-window", wid, str(output / name)], check=True)


def props(path):
    return dict(line.split("=", 1) for line in path.read_text().splitlines() if "=" in line and not line.startswith("#"))


def wait(check, message, timeout=240):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        result = check()
        if result:
            return result
        time.sleep(.2)
    raise AssertionError(message)


results = []
for version in versions:
    loader = "forge" if version.startswith("1.8") else "fabric"
    major = 25 if version == "26.3" else 21 if version.startswith("1.21") else 17 if version.startswith("1.20") else 8
    java = os.environ[f"CAIRN_MENU_JAVA_{major}"]
    env = {**os.environ, "CAIRN_HOME": str(work), "E2E_PLAYER": "Menu" + version.replace(".", ""), "E2E_JAVA": java,
           "E2E_CAIRN": "1", "E2E_FRIENDS": "1", "E2E_WIDTH": "1920", "E2E_HEIGHT": "1080", "E2E_HOLD": "1800"}
    if version == "1.16.5":
        env["E2E_MENU_KEY"] = "77"
    log = work / ("menu-" + version + ".log")
    with log.open("w") as stream:
        process = subprocess.Popen([str(root / "src-tauri/target/debug/examples/e2e"), version, loader], stdout=stream, stderr=subprocess.STDOUT, env=env, start_new_session=True)
        try:
            def ready():
                if process.poll() is not None:
                    raise AssertionError(f"{version} exited during startup; see {log}")
                text = log.read_text(errors="replace")
                return window() if "Created:" in text and "prepared " in text else None

            wid = wait(ready, f"{version} did not load its menu")
            time.sleep(4)
            instance = Path(log.read_text().split(" at ", 1)[1].splitlines()[0])
            config = instance / "config/cairn-client.properties"
            xdo("windowraise", wid, "windowfocus", "--sync", wid, "key", "m" if version == "1.16.5" else "Shift_R")
            time.sleep(.5)
            click(wid, 380, 180)
            wait(lambda: props(config).get("menu.page") == "modules", "Modules navigation did not respond", 10)
            capture(wid, f"modules-{version}.png")
            click(wid, 490, 455)
            wait(lambda: props(config).get("FPS.enabled") == "false", "FPS switch did not respond", 10)
            click(wid, 490, 455)
            wait(lambda: props(config).get("FPS.enabled") == "true", "FPS switch did not enable again", 10)
            click(wid, 1280, 260)
            xdo("type", "--clearmodifiers", "--delay", 80, "zoom")
            time.sleep(.4)
            click(wid, 490, 455)
            wait(lambda: props(config).get("ZOOM.enabled") == "false", "Search text or the filtered switch did not respond", 10)
            capture(wid, f"search-{version}.png")
            click(wid, 340, 330)
            capture(wid, f"settings-{version}.png")
            click(wid, 1610, 120)
            capture(wid, f"closed-{version}.png")
            results.append({"version": version, "loader": loader, "navigation": "passed", "mouseToggle": "passed", "textSearch": "passed", "settingsAndClose": "captured for visual inspection"})
            print(json.dumps(results[-1]), flush=True)
            (output / "results.json").write_text(json.dumps(results, indent=2) + "\n")
        finally:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            process.wait(timeout=20)
            time.sleep(.5)
