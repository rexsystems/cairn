#!/usr/bin/env python3
"""Draws every bundled Cairn cosmetic from the logo geometry in src-tauri/icons/icon.svg.

Run from the repo root: python3 client-mod/cosmetics/generate.py
Writes the mod textures, the shared catalog, launcher previews and the preview sheets.
"""
import colorsys, json, math, os, shutil
from PIL import Image, ImageDraw, ImageFilter

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MOD = os.path.join(ROOT, "client-mod/common/src/main/resources/assets/cairn_client/cosmetics")
PUBLIC = os.path.join(ROOT, "public/cosmetics")
PREVIEWS = os.path.join(ROOT, "client-mod/cosmetics/previews")
SS = 4  # supersampling for smooth logo edges
S = 8   # cape texture scale: 512x256 per frame, the front panel is 80x128
U = S // 4  # design unit, so offsets below scale with S

# Logo geometry from icon.svg (128 unit viewBox): four stacked stones and the play mark.
STONES = [(30, 18, 68, 22, 11), (22, 44, 52, 20, 10), (22, 68, 52, 20, 10), (30, 92, 72, 22, 11)]
MARK = [(75.5, 49.5), (75.5, 79.0), (99.5, 64.0)]


def rgb(hex_color, alpha=255):
    hex_color = hex_color.lstrip("#")
    return tuple(int(hex_color[i:i + 2], 16) for i in (0, 2, 4)) + (alpha,)


def logo(size, stone, mark, tile=None, outline=None):
    """Renders the logo into a size x size RGBA image; colors are #rrggbb strings or RGBA tuples."""
    big = size * SS
    image = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    k = big / 128
    col = lambda c: rgb(c) if isinstance(c, str) else c
    if tile:
        draw.rounded_rectangle([0, 0, big - 1, big - 1], radius=28 * k, fill=col(tile))
    if outline:
        grow = 5 * k
        for x, y, w, h, r in STONES:
            draw.rounded_rectangle([x * k - grow, y * k - grow, (x + w) * k + grow, (y + h) * k + grow], radius=(r + 5) * k, fill=col(outline))
        draw.polygon([(px * k, py * k) for px, py in MARK], fill=col(outline), outline=col(outline), width=int(grow * 2))
    for x, y, w, h, r in STONES:
        draw.rounded_rectangle([x * k, y * k, (x + w) * k, (y + h) * k], radius=r * k, fill=col(stone))
    draw.polygon([(px * k, py * k) for px, py in MARK], fill=col(mark))
    return image.resize((size, size), Image.LANCZOS)


def gradient(w, h, top, bottom, angle=0.0, offset=0.0):
    """Linear gradient; offset scrolls a repeating gradient for animated capes."""
    image = Image.new("RGBA", (w, h))
    px = image.load()
    a, b = rgb(top) if isinstance(top, str) else top, rgb(bottom) if isinstance(bottom, str) else bottom
    ca, sa = math.cos(angle), math.sin(angle)
    span = abs(w * sa) + abs(h * ca) or 1
    for y in range(h):
        for x in range(w):
            t = ((x * sa + y * ca) / span + offset) % 1.0 if offset else (x * sa + y * ca) / span
            t = 1 - abs(1 - 2 * t) if offset else min(1, max(0, t))
            px[x, y] = tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(4))
    return image


def cape_frame(background, design, edge):
    """Builds one 64x32-layout cape at scale S: the design goes on the outer face at (1,1)."""
    w, h = 64 * S, 32 * S
    image = background(w, h)
    draw = ImageDraw.Draw(image)
    front = (1 * S, 1 * S, 11 * S, 17 * S)
    design(image, front)
    # Thin accent frame around the outer face so the cape reads well at a distance.
    draw.rectangle([front[0], front[1], front[2] - 1, front[3] - 1], outline=rgb(edge), width=U)
    # Inner face and edges stay calm and dark so the design only shows outside.
    inner = Image.new("RGBA", (10 * S, 16 * S), rgb("#16130f"))
    inner.paste(logo(6 * S, "#3a342c", "#4d4336"), (2 * S, 5 * S), logo(6 * S, "#3a342c", "#4d4336"))
    image.paste(inner, (12 * S, 1 * S))
    for x0, y0, x1, y1 in [(0, 1, 1, 17), (11, 1, 12, 17), (1, 0, 11, 1), (11, 0, 21, 1)]:
        draw.rectangle([x0 * S, y0 * S, x1 * S - 1, y1 * S - 1], fill=rgb(edge))
    return image


def place_logo(image, front, icon, dy=0):
    x0, y0, x1, y1 = front
    image.alpha_composite(icon, (x0 + (x1 - x0 - icon.width) // 2, y0 + (y1 - y0 - icon.height) // 2 - 6 * U + dy))


def stripes(image, front, color, count=3):
    x0, y0, x1, y1 = front
    draw = ImageDraw.Draw(image)
    for i in range(count):
        y = y1 - (12 + i * 4) * U
        draw.rectangle([x0 + 6 * U, y, x1 - 7 * U, y + U], fill=rgb(color, 200 - i * 60))


def stars(image, front, seed, color="#f2ede5"):
    x0, y0, x1, y1 = front
    state = seed
    for _ in range(28):
        state = (state * 1103515245 + 12345) & 0x7FFFFFFF
        x = x0 + 2 + state % (x1 - x0 - 4)
        state = (state * 1103515245 + 12345) & 0x7FFFFFFF
        y = y0 + 2 + state % (y1 - y0 - 4)
        ImageDraw.Draw(image).rectangle([x, y, x + U - 1, y + U - 1], fill=rgb(color, 120 + state % 135))


STATIC_CAPES = [
    # id, name, background top, bottom, stone, mark, edge, extra
    ("classic", "Cairn Classic", "#2b2722", "#1b1815", "#f2ede5", "#e8a33d", "#e8a33d", "stripes"),
    ("moss", "Moss", "#3c4835", "#1c211c", "#c6db82", "#f1f0e8", "#c6db82", "stripes"),
    ("ember", "Ember", "#7a2614", "#2a0d08", "#ffd8a8", "#ff8a3d", "#ff8a3d", "stripes"),
    ("glacier", "Glacier", "#2f5f86", "#0f2236", "#e8f6ff", "#7fd3ff", "#9fdcff", "stars"),
    ("midnight", "Midnight", "#22204a", "#0b0a1c", "#d9d2ff", "#b48cff", "#8c7bff", "stars"),
]
STAFF_CAPE = ("staff", "Cairn Staff", "#2a2a2a", "#0e0e0e", "#ffe7a3", "#ff4d5e", "#ffcf5a", "staff")


def static_cape(spec):
    cid, _, top, bottom, stone, mark, edge, extra = spec
    def design(image, front):
        if extra == "stars": stars(image, front, len(cid) * 7919)
        place_logo(image, front, logo(30 * U, stone, mark))
        if extra == "stripes": stripes(image, front, edge)
        if extra == "staff":
            x0, y0, x1, y1 = front
            draw = ImageDraw.Draw(image)
            for i, color in enumerate(("#ffcf5a", "#ff4d5e", "#ffcf5a")):
                draw.rectangle([x0 + 4 * U, y1 - (14 - i * 3) * U, x1 - 5 * U, y1 - (13 - i * 3) * U], fill=rgb(color))
    return cape_frame(lambda w, h: gradient(w, h, top, bottom), design, edge)


def pulse_frames(count=16):
    frames = []
    for i in range(count):
        t = (math.sin(i / count * 2 * math.pi) + 1) / 2
        def design(image, front, t=t):
            glow = Image.new("RGBA", image.size, (0, 0, 0, 0))
            x0, y0, x1, y1 = front
            cx, cy, r = (x0 + x1) // 2, (y0 + y1) // 2 - 6 * U, (12 + 6 * t) * U
            ImageDraw.Draw(glow).ellipse([cx - r, cy - r, cx + r, cy + r], fill=rgb("#e8a33d", int(90 + 120 * t)))
            image.alpha_composite(glow.filter(ImageFilter.GaussianBlur((4 + 3 * t) * U)))
            mark = tuple(int(a + (b - a) * t) for a, b in zip(rgb("#e8a33d"), rgb("#fff1c9")))
            place_logo(image, front, logo(30 * U, "#f2ede5", mark))
            stripes(image, front, "#e8a33d")
        frames.append(cape_frame(lambda w, h: gradient(w, h, "#2b2722", "#141210"), design, "#e8a33d"))
    return frames


def hue(hex_color, shift, sat=1.0):
    r, g, b, _ = rgb(hex_color)
    h, l, s = colorsys.rgb_to_hls(r / 255, g / 255, b / 255)
    r, g, b = colorsys.hls_to_rgb((h + shift) % 1, l, min(1, s * sat))
    return (int(r * 255), int(g * 255), int(b * 255), 255)


def spectrum_frames(count=20):
    frames = []
    for i in range(count):
        shift = i / count
        def design(image, front, shift=shift):
            place_logo(image, front, logo(30 * U, "#f6f2ec", hue("#ff5f6d", shift)))
            stripes(image, front, "#ffffff")
        background = lambda w, h, shift=shift: gradient(w, h, hue("#5a1e8c", shift), hue("#0e0b20", shift), 0.4)
        frames.append(cape_frame(background, design, "#%02x%02x%02x" % hue("#ff5f6d", shift)[:3]))
    return frames


def flow_frames(count=16):
    frames = []
    for i in range(count):
        def design(image, front):
            place_logo(image, front, logo(30 * U, "#effff8", "#3dffb4", outline="#0b2a22"))
        background = lambda w, h, i=i: gradient(w, h, "#0f8f74", "#10264a", 0.7, i / count)
        frames.append(cape_frame(background, design, "#3dffb4"))
    return frames


ANIMATED_CAPES = [
    ("pulse", "Pulse", pulse_frames, 100),
    ("spectrum", "Spectrum", spectrum_frames, 80),
    ("flow", "Flow", flow_frames, 90),
]
BADGES = [
    # id, name, stone, mark, role needed to equip
    ("classic", "Classic", "#f2ede5", "#e8a33d", None),
    ("moss", "Moss", "#c6db82", "#f1f0e8", None),
    ("ember", "Ember", "#ffd8a8", "#ff6a2b", None),
    ("glacier", "Glacier", "#e8f6ff", "#4fc3ff", None),
    ("amethyst", "Amethyst", "#e6dcff", "#a66bff", None),
    ("rose", "Rose", "#ffe1ea", "#ff5f8a", None),
    ("gold", "Staff gold", "#ffe7a3", "#ff4d5e", "staff"),
]


def strip(frames):
    w, h = frames[0].size
    out = Image.new("RGBA", (w, h * len(frames)))
    for i, frame in enumerate(frames): out.paste(frame, (0, i * h))
    return out


def front(frame, scale=2):
    face = frame.crop((1 * S, 1 * S, 11 * S, 17 * S))
    return face.resize((face.width * scale, face.height * scale), Image.NEAREST)


def card(face, size=(240, 320)):
    image = Image.new("RGBA", size, rgb("#1c211c"))
    shadow = Image.new("RGBA", size, (0, 0, 0, 0))
    x, y = (size[0] - face.width) // 2, (size[1] - face.height) // 2
    ImageDraw.Draw(shadow).rectangle([x + 6, y + 8, x + face.width + 6, y + face.height + 8], fill=(0, 0, 0, 140))
    image.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(6)))
    image.alpha_composite(face, (x, y))
    return image


def main():
    for d in (MOD + "/capes", MOD + "/badges", PUBLIC, PREVIEWS):
        shutil.rmtree(d, ignore_errors=True)
        os.makedirs(d)
    catalog = {"version": 1, "capes": [], "badges": []}
    sheet = []
    for spec in STATIC_CAPES + [STAFF_CAPE]:
        frame = static_cape(spec)
        frame.save(f"{MOD}/capes/{spec[0]}.png", optimize=True)
        card(front(frame)).save(f"{PUBLIC}/cape-{spec[0]}.png", optimize=True)
        catalog["capes"].append({"id": spec[0], "name": spec[1], "frames": 1, "frameMs": 0, "role": "staff" if spec is STAFF_CAPE else None})
        sheet.append((spec[1], front(frame)))
    for cid, name, make, frame_ms in ANIMATED_CAPES:
        frames = make()
        strip(frames).save(f"{MOD}/capes/{cid}.png", optimize=True)
        cards = [card(front(f)) for f in frames]
        cards[0].save(f"{PUBLIC}/cape-{cid}.png", optimize=True)
        cards[0].save(f"{PREVIEWS}/cape-{cid}.gif", save_all=True, append_images=cards[1:], duration=frame_ms, loop=0, disposal=2)
        catalog["capes"].append({"id": cid, "name": name, "frames": len(frames), "frameMs": frame_ms, "role": None})
        sheet.append((name + " (animated)", front(frames[0])))
    for bid, name, stone, mark, role in BADGES:
        logo(32, stone, mark).save(f"{MOD}/badges/{bid}.png", optimize=True)
        logo(96, stone, mark, tile="#221f1b").save(f"{PUBLIC}/badge-{bid}.png", optimize=True)
        catalog["badges"].append({"id": bid, "name": name, "role": role})
    # Contact sheets for review.
    face_w, face_h = sheet[0][1].size
    cols = 5
    rows = (len(sheet) + cols - 1) // cols
    out = Image.new("RGBA", (cols * (face_w + 40) + 40, rows * (face_h + 70) + 40), rgb("#14181a"))
    draw = ImageDraw.Draw(out)
    for i, (name, face) in enumerate(sheet):
        x, y = 40 + i % cols * (face_w + 40), 40 + i // cols * (face_h + 70)
        out.alpha_composite(face, (x, y))
        draw.text((x, y + face_h + 10), name, fill=rgb("#f1f0e8"))
    out.save(f"{PREVIEWS}/capes.png")
    badges = Image.new("RGBA", (len(BADGES) * 140 + 20, 190), rgb("#14181a"))
    draw = ImageDraw.Draw(badges)
    for i, (bid, name, stone, mark, role) in enumerate(BADGES):
        badges.alpha_composite(logo(96, stone, mark, tile="#221f1b"), (20 + i * 140 + 12, 20))
        draw.text((20 + i * 140 + 12, 130), name + (" (staff)" if role else ""), fill=rgb("#f1f0e8"))
        badges.alpha_composite(logo(16, stone, mark), (20 + i * 140 + 12, 155))
        draw.text((20 + i * 140 + 32, 158), "Steve", fill=rgb("#ffffff"))
    badges.save(f"{PREVIEWS}/badges.png")
    for path in (os.path.join(MOD, "catalog.json"), os.path.join(ROOT, "cosmetics-server/catalog.json"), os.path.join(ROOT, "src/lib/cosmeticsCatalog.json")):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as f: json.dump(catalog, f, indent=2); f.write("\n")


if __name__ == "__main__":
    main()
