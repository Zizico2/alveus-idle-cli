#!/usr/bin/env python3
"""Generate procedural overview-map tile art (terrain tiles are hand-authored).

Compost bin placement uses two Tiled tile layers (same pattern as interior Floor + Objects):
  - Terrain: grass (gid 3) underfoot
  - Objects: compost_bin (gid 4) on top — PNG is RGBA transparent outside the bin art
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "assets" / "maps" / "overview"

TILE = 32

# Brown compost-bin palette (matches existing committed asset).
COMPOST_LID = (100, 75, 45)
COMPOST_BODY = (80, 60, 40)


def new_tile() -> Image.Image:
    return Image.new("RGBA", (TILE, TILE), (0, 0, 0, 0))


def draw_compost_bin() -> Image.Image:
    """Top-down compost bin: narrow lid slab over a wider body."""
    img = new_tile()
    draw = ImageDraw.Draw(img)
    # Lid (lighter brown)
    draw.rectangle((6, 4, 25, 7), fill=COMPOST_LID)
    # Body (darker brown)
    draw.rectangle((4, 8, 27, 26), fill=COMPOST_BODY)
    return img


def save_tile(img: Image.Image, name: str) -> None:
    path = OUT_DIR / name
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path)
    print(f"  wrote {path.relative_to(ROOT)}")


def main() -> None:
    print("Generating overview procedural tiles...")
    save_tile(draw_compost_bin(), "compost_bin.png")
    print("Done.")


if __name__ == "__main__":
    main()
