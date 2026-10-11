#!/usr/bin/env python3
"""Trace a logo PNG to a stacked color SVG via Inkscape object-trace."""
from __future__ import annotations

import re
import subprocess
import sys
import tempfile
from pathlib import Path


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: png_to_svg_logo.py input.png output.svg", file=sys.stderr)
        return 2
    src = Path(sys.argv[1]).resolve()
    dst = Path(sys.argv[2]).resolve()
    with tempfile.TemporaryDirectory() as td:
        work = Path(td)
        logo = work / "logo.png"
        subprocess.check_call(
            ["magick", str(src), "-resize", "1024x1024", str(logo)]
        )
        raw = work / "raw.svg"
        # scans,smooth,stack,remove_bg,speckles,smooth_corners,optimize
        subprocess.check_call(
            [
                "inkscape",
                str(logo),
                f"--actions=select-all;object-trace:12,1,1,1,2,1,1;export-filename:{raw};export-type:svg;export-do",
            ]
        )
        text = raw.read_text()
        text = re.sub(r"<image[\s\S]*?/>", "", text)
        text = re.sub(r"<image[\s\S]*?</image>", "", text)
        vectors = work / "vectors.svg"
        vectors.write_text(text)
        plain = work / "plain.svg"
        subprocess.check_call(
            ["inkscape", str(vectors), "--export-plain-svg", f"--export-filename={plain}"]
        )
        s = plain.read_text()
        s = re.sub(r'\s(inkscape|sodipodi):[A-Za-z0-9:\-]+="[^"]*"', "", s)
        s = re.sub(r'\sxmlns:(inkscape|sodipodi)="[^"]*"', "", s)
        s = re.sub(r"<!--.*?-->", "", s, flags=re.S)
        if "<title>" not in s:
            s = re.sub(r"(<svg[^>]*>)", r"\1\n  <title>Cairn</title>", s, count=1)
        s = re.sub(r"\n{3,}", "\n\n", s)
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_text(s)
        print(f"wrote {dst} ({dst.stat().st_size} bytes, {len(re.findall(r'<path', s))} paths)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
