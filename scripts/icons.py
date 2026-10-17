#!/usr/bin/env python3
"""Builds the desktop icons from the Android app's vector launcher icon (the marbled endpaper and the page).

    scripts/icons.py [path/to/forzac/app/src/main/res]      (default: ../forzac next to this repository)

The Android adaptive icon is 108 x 108 dp, of which a launcher shows the central 72 (as the store and site icons do).
That square, converted path for path to SVG, is drawn into a desktop shape:
- Linux: a rounded square with a small margin -> data/icons/hicolor/scalable/apps/app.truenrth.forzac.svg and PNGs
  from 16 to 512 px in data/icons/hicolor/<n>x<n>/apps/; the monochrome layer (an open book in strokes) as the
  symbolic icon GNOME draws in the foreground colour -> data/icons/hicolor/symbolic/apps/app.truenrth.forzac-symbolic.svg;
- macOS: the macOS icon grid (an 824 px rounded square with continuous corners centred on a 1024 px canvas, a soft
  shadow below) -> macos/Forzac.icns.
Needs rsvg-convert (librsvg) and, for the .icns, macOS's iconutil. The outputs are committed; packages and
macos/build.sh only copy them.
"""
import math
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
A = "{http://schemas.android.com/apk/res/android}"
APP_ID = "app.truenrth.forzac"
LINUX_SIZES = (16, 22, 24, 32, 48, 64, 128, 256, 512)


def paint(value: str) -> str:
    """#AARRGGBB or #RRGGBB -> SVG colour and opacity attributes' values."""
    v = value.lstrip("#")
    alpha = int(v[:2], 16) / 255 if len(v) == 8 else 1.0
    return f"#{v[-6:]}", f"{alpha:.3g}"


def art(res: Path) -> str:
    """The background and foreground layers as SVG elements in the Android viewport (0..108)."""
    out, gradients = [], 0
    for layer in ("background", "foreground"):
        for path in ET.parse(res / f"drawable/ic_launcher_{layer}.xml").getroot().iter("path"):
            attrs = [f'd="{path.get(A + "pathData")}"']
            gradient = path.find(".//gradient")
            if gradient is not None:
                assert gradient.get(A + "type") == "linear"
                gradients += 1
                stops = "".join(
                    '<stop offset="{}" stop-color="{}" stop-opacity="{}"/>'.format(i.get(A + "offset"), *paint(i.get(A + "color")))
                    for i in gradient.findall("item")
                )
                ends = " ".join(f'{k}="{gradient.get(A + n)}"' for k, n in (("x1", "startX"), ("y1", "startY"), ("x2", "endX"), ("y2", "endY")))
                out.append(f'<linearGradient id="g{gradients}" gradientUnits="userSpaceOnUse" {ends}>{stops}</linearGradient>')
                attrs.append(f'fill="url(#g{gradients})"')
            else:
                fill, opacity = paint(path.get(A + "fillColor"))
                attrs.append(f'fill="{fill}"' + (f' fill-opacity="{opacity}"' if opacity != "1" else ""))
            if path.get(A + "strokeColor"):
                stroke, opacity = paint(path.get(A + "strokeColor"))
                attrs.append(f'stroke="{stroke}" stroke-opacity="{opacity}" stroke-width="{path.get(A + "strokeWidth")}"')
            out.append(f"<path {' '.join(attrs)}/>")
    return "\n".join(out)


def symbolic(res: Path) -> str:
    """The monochrome layer as a 16 px symbolic icon: its strokes in the Android group's transform, the viewBox cut to
    the book (the drawing spans about 33..76 x 36..72 of the 108 dp viewport)."""
    root = ET.parse(res / "drawable/ic_launcher_monochrome.xml").getroot()
    group = root.find("group")
    names = {"pivotX": 0, "pivotY": 0, "scaleX": 1, "scaleY": 1, "translateX": 0, "translateY": 0}
    g = {k: float(group.get(A + k, default)) for k, default in names.items()}
    # Android applies -pivot, scale, then translate + pivot.
    transform = (
        f"translate({g['translateX'] + g['pivotX']:g} {g['translateY'] + g['pivotY']:g}) "
        f"scale({g['scaleX']:g} {g['scaleY']:g}) translate({-g['pivotX']:g} {-g['pivotY']:g})"
    )
    paths = []
    for path in group.iter("path"):
        cap, join = path.get(A + "strokeLineCap", "butt"), path.get(A + "strokeLineJoin", "miter")
        paths.append(
            f'<path d="{path.get(A + "pathData")}" fill="none" stroke="#2e3436" stroke-width="{path.get(A + "strokeWidth")}" '
            f'stroke-linecap="{cap}" stroke-linejoin="{join}"/>'
        )
    body = "\n".join(paths)
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="31 31 46 46">\n'
        f'<g transform="{transform}">\n{body}\n</g>\n</svg>\n'
    )


def squircle(x: float, y: float, size: float, n: float = 5.0, steps: int = 256) -> str:
    """A superellipse |x|^n + |y|^n = 1: the continuous-corner rounded square of macOS icons."""
    r, c = size / 2, (x + size / 2, y + size / 2)
    points = []
    for i in range(steps):
        t = 2 * math.pi * i / steps
        ct, st = math.cos(t), math.sin(t)
        points.append((c[0] + r * math.copysign(abs(ct) ** (2 / n), ct), c[1] + r * math.copysign(abs(st) ** (2 / n), st)))
    return "M" + " ".join(f"{px:.2f},{py:.2f}" for px, py in points) + "Z"


def icon(content: str, canvas: int, body: float, shape: str, shadow: bool) -> str:
    """The visible 72 dp of the art (18..90) scaled into a `body`-sized square centred on the canvas, clipped by
    `shape`."""
    offset = (canvas - body) / 2
    scale = body / 72
    defs = f'<clipPath id="shape"><path d="{shape}"/></clipPath>'
    under = ""
    if shadow:
        defs += (
            '<filter id="shadow" x="-10%" y="-10%" width="120%" height="125%">'
            '<feGaussianBlur in="SourceAlpha" stdDeviation="9"/><feOffset dy="10"/>'
            '<feComponentTransfer><feFuncA type="linear" slope="0.3"/></feComponentTransfer></filter>'
        )
        under = f'<path d="{shape}" filter="url(#shadow)"/>\n'
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{canvas}" height="{canvas}" viewBox="0 0 {canvas} {canvas}">\n'
        f"<defs>{defs}</defs>\n{under}"
        f'<g clip-path="url(#shape)"><g transform="translate({offset - 18 * scale:.4f} {offset - 18 * scale:.4f}) scale({scale:.6f})">\n'
        f"{content}\n</g></g>\n</svg>\n"
    )


def render(svg: Path, png: Path, size: int) -> None:
    png.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["rsvg-convert", "-w", str(size), "-h", str(size), "-o", str(png), str(svg)], check=True)


def main() -> None:
    res = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT.parent / "forzac/app/src/main/res"
    content = art(res)

    # Linux: 128 px design grid, a 6 px margin, 20 px corners.
    hicolor = ROOT / "data/icons/hicolor"
    linux = hicolor / f"scalable/apps/{APP_ID}.svg"
    linux.parent.mkdir(parents=True, exist_ok=True)
    rounded = "M26,6H102A20,20 0 0 1 122,26V102A20,20 0 0 1 102,122H26A20,20 0 0 1 6,102V26A20,20 0 0 1 26,6Z"
    linux.write_text(icon(content, 128, 116, rounded, shadow=False))
    for size in LINUX_SIZES:
        render(linux, hicolor / f"{size}x{size}/apps/{APP_ID}.png", size)
    mono = hicolor / f"symbolic/apps/{APP_ID}-symbolic.svg"
    mono.parent.mkdir(parents=True, exist_ok=True)
    mono.write_text(symbolic(res))

    # macOS: Apple's 1024 px grid, an 824 px body.
    with tempfile.TemporaryDirectory() as tmp:
        mac = Path(tmp) / "icon.svg"
        mac.write_text(icon(content, 1024, 824, squircle(100, 100, 824), shadow=True))
        iconset = Path(tmp) / "Forzac.iconset"
        for size in (16, 32, 128, 256, 512):
            render(mac, iconset / f"icon_{size}x{size}.png", size)
            render(mac, iconset / f"icon_{size}x{size}@2x.png", size * 2)
        if shutil.which("iconutil"):
            subprocess.run(["iconutil", "-c", "icns", "-o", str(ROOT / "macos/Forzac.icns"), str(iconset)], check=True)
        else:
            print("no iconutil (not macOS): macos/Forzac.icns left as it is", file=sys.stderr)
        preview = ROOT / "target/icons"
        preview.mkdir(parents=True, exist_ok=True)
        shutil.copy(iconset / "icon_512x512@2x.png", preview / "macos-1024.png")


if __name__ == "__main__":
    main()
