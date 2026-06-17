#!/usr/bin/env python3
"""
generate_hi_dpi_icons.py — v0.3.15.x PHASE-4 task 4.2.

Generate @2x.png high-DPI variants of every PNG in icons/.

Inputs:  icons/<name>.png
Outputs: icons/<name>@2x.png   (200% of original, LANCZOS resample)

Idempotent: existing @2x.png files are skipped (delete the @2x file
to force a rebuild). Non-PNG files in icons/ (.ico, .rc, .manifest)
are ignored.

Usage:
    python scripts/generate_hi_dpi_icons.py [--force]

Optional --force overwrites existing @2x.png files.
"""
import argparse
import sys
from pathlib import Path

from PIL import Image


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--force",
        action="store_true",
        help="overwrite existing @2x.png files",
    )
    parser.add_argument(
        "--icons-dir",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "icons",
        help="path to the icons/ directory (default: ../icons relative to script)",
    )
    args = parser.parse_args()

    icons_dir: Path = args.icons_dir
    if not icons_dir.is_dir():
        print(f"error: {icons_dir} is not a directory", file=sys.stderr)
        return 1

    # Index every PNG and skip ones whose stem already ends in @2x so
    # we don't double-process a previously-generated variant.
    pngs = sorted(
        p for p in icons_dir.iterdir()
        if p.suffix.lower() == ".png" and not p.stem.endswith("@2x")
    )

    if not pngs:
        print(f"no PNGs found in {icons_dir}")
        return 0

    generated = 0
    skipped = 0
    for src in pngs:
        dst = src.with_name(f"{src.stem}@2x.png")
        if dst.exists() and not args.force:
            skipped += 1
            print(f"  skip  {dst.name}  (exists; pass --force to overwrite)")
            continue
        with Image.open(src) as img:
            w, h = img.size
            new_size = (max(1, w * 2), max(1, h * 2))
            # LANCZOS resample produces the cleanest 2x upscale; it
            # is what Qt expects when devicePixelRatio == 2.
            upscaled = img.resize(new_size, Image.Resampling.LANCZOS)
            # Preserve the original mode where possible; RGBA -> RGBA
            # (no alpha flattening), palette -> RGBA, L -> L, etc.
            upscaled.save(dst, format="PNG", optimize=True)
        generated += 1
        print(f"  wrote {dst.name}  ({w}x{h} -> {new_size[0]}x{new_size[1]})")

    print(f"\ndone: {generated} generated, {skipped} skipped, "
          f"{len(pngs)} total PNGs in {icons_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
