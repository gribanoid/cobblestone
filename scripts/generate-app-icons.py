#!/usr/bin/env python3
"""Generate app icons from cobblestone.svg (pixel-art source).

  cobblestone.svg → 1024² → macOS layout (824/1024 squircle) → icon.icns + icon.png
                  → 128²  → frontend/packages/ui/public/cobblestone.png
"""
from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FRONTEND = ROOT / "frontend"
ICONS = ROOT / "crates/desktop/src-tauri/icons"
SPRITE_SVG = ICONS / "cobblestone.svg"
ICNS = ICONS / "icon.icns"
ICON_PNG = ICONS / "icon.png"
RASTERIZE = FRONTEND / "scripts/rasterize-icon.mjs"
APPLY_MACOS_ICON = ROOT / "scripts/apply-macos-icon.swift"
UI_PUBLIC = FRONTEND / "packages/ui/public/cobblestone.png"

CANVAS = 1024
TAURI_ICON = 512
UI_ICON = 128

ICON_SIZES: list[tuple[str, int]] = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
]


def rasterize_svg(svg: Path, out_png: Path, size: int) -> None:
    subprocess.run(
        ["node", str(RASTERIZE), str(svg), str(out_png), str(size)],
        cwd=FRONTEND,
        check=True,
    )


def apply_macos_layout(src: Path, dest: Path, canvas: int) -> None:
    subprocess.run(
        ["swift", str(APPLY_MACOS_ICON), str(src), str(dest), str(canvas)],
        check=True,
    )


def build_icns(src_png: Path, icns_path: Path) -> None:
    if shutil.which("sips") is None or shutil.which("iconutil") is None:
        print("sips/iconutil not found — skipping .icns (non-macOS?)", file=sys.stderr)
        return

    iconset_dir = icns_path.with_suffix(".iconset")
    if iconset_dir.exists():
        shutil.rmtree(iconset_dir)
    iconset_dir.mkdir()

    try:
        for name, size in ICON_SIZES:
            out = iconset_dir / name
            subprocess.run(
                ["sips", "-z", str(size), str(size), str(src_png), "--out", str(out)],
                check=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        subprocess.run(
            ["iconutil", "-c", "icns", str(iconset_dir), "--output", str(icns_path)],
            check=True,
        )
    finally:
        if iconset_dir.exists():
            shutil.rmtree(iconset_dir)

    print(f"Wrote {icns_path.relative_to(ROOT)}")


def main() -> int:
    if not SPRITE_SVG.is_file():
        print(f"Missing {SPRITE_SVG}", file=sys.stderr)
        return 1
    if not RASTERIZE.is_file():
        print(f"Missing {RASTERIZE}", file=sys.stderr)
        return 1
    if not APPLY_MACOS_ICON.is_file():
        print(f"Missing {APPLY_MACOS_ICON}", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory(prefix="cobblestone-icon-") as tmp:
        tmp_path = Path(tmp)
        raster = tmp_path / "sprite-1024.png"
        layout = tmp_path / "layout-1024.png"

        rasterize_svg(SPRITE_SVG, raster, CANVAS)
        apply_macos_layout(raster, layout, CANVAS)
        build_icns(layout, ICNS)
        apply_macos_layout(raster, ICON_PNG, TAURI_ICON)

    UI_PUBLIC.parent.mkdir(parents=True, exist_ok=True)
    rasterize_svg(SPRITE_SVG, UI_PUBLIC, UI_ICON)
    print(f"Wrote {ICON_PNG.relative_to(ROOT)} ({TAURI_ICON}², macOS layout)")
    print(f"Wrote {UI_PUBLIC.relative_to(ROOT)} ({UI_ICON}² pixel-art)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
