#!/usr/bin/env python3
"""Generate interior tile art and Tiled map files for Nutrition House and Push Pop Enclosure."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Callable

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "assets" / "maps" / "interiors"
TILES_DIR = OUT_DIR / "tiles"

TILE = 32
OBSTACLE_PROP = """   <properties>
    <property name="obstacle" type="class" propertytype="alveus_idle_cli::components::Obstacle">
     <properties/>
    </property>
   </properties>"""

# Palette derived from room.rs Color::srgb values and nutrition_house.png cottage style.
PALETTE = {
    "outline": (45, 35, 28),
    "wood_dark": (56, 41, 31),
    "wood_mid": (89, 64, 45),
    "wood_light": (122, 88, 58),
    "cream": (235, 220, 195),
    "cream_shadow": (210, 190, 165),
    "wall_trim": (100, 72, 48),
    "door": (102, 64, 38),
    "door_knob": (180, 150, 60),
    "teal": (26, 128, 102),
    "teal_dark": (18, 96, 78),
    "fridge_body": (191, 199, 204),
    "fridge_dark": (150, 158, 165),
    "fridge_handle": (120, 125, 130),
    "chest": (153, 102, 26),
    "chest_dark": (110, 72, 18),
    "sand": (184, 158, 115),
    "sand_dark": (158, 132, 92),
    "sand_pebble": (130, 108, 78),
    "fence": (115, 97, 71),
    "fence_dark": (85, 70, 50),
    "gate": (128, 102, 71),
    "shelter": (102, 82, 56),
    "shelter_dark": (72, 58, 38),
    "dish": (140, 115, 77),
    "dish_inner": (90, 75, 55),
    "food_green": (80, 140, 60),
}


def new_tile() -> Image.Image:
    return Image.new("RGBA", (TILE, TILE), (0, 0, 0, 0))


def save_tile(img: Image.Image, name: str) -> None:
    path = TILES_DIR / name
    img.save(path)
    print(f"  wrote {path.relative_to(ROOT)}")


def draw_wood_floor(variant: int = 0) -> Image.Image:
    img = new_tile()
    draw = ImageDraw.Draw(img)
    base = PALETTE["wood_dark"] if variant == 0 else (62, 46, 34)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), fill=base)
    for y in range(4, TILE, 8):
        draw.line((0, y, TILE - 1, y), fill=PALETTE["wood_mid"], width=1)
    for x in range(0, TILE, 8):
        shade = PALETTE["wood_light"] if (x // 8 + variant) % 2 == 0 else PALETTE["wood_mid"]
        draw.line((x, 0, x, TILE - 1), fill=shade, width=1)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), outline=PALETTE["outline"], width=1)
    return img


def draw_wall() -> Image.Image:
    img = new_tile()
    draw = ImageDraw.Draw(img)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), fill=PALETTE["cream"])
    for y in range(6, TILE, 10):
        draw.line((0, y, TILE - 1, y), fill=PALETTE["cream_shadow"], width=1)
    draw.rectangle((0, 0, TILE - 1, 5), fill=PALETTE["wall_trim"])
    draw.rectangle((0, TILE - 6, TILE - 1, TILE - 1), fill=PALETTE["wall_trim"])
    draw.rectangle((0, 0, TILE - 1, TILE - 1), outline=PALETTE["outline"], width=1)
    return img


def draw_wood_door() -> Image.Image:
    img = new_tile()
    draw = ImageDraw.Draw(img)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), fill=PALETTE["wood_dark"])
    draw.rectangle((6, 2, TILE - 7, TILE - 3), fill=PALETTE["door"])
    for x in range(10, TILE - 10, 4):
        draw.line((x, 4, x, TILE - 5), fill=PALETTE["wood_mid"], width=1)
    draw.ellipse((TILE - 12, TILE // 2 - 2, TILE - 8, TILE // 2 + 2), fill=PALETTE["door_knob"])
    draw.rectangle((0, 0, TILE - 1, TILE - 1), outline=PALETTE["outline"], width=1)
    return img


def draw_prep_table() -> Image.Image:
    img = new_tile()
    draw = ImageDraw.Draw(img)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), fill=PALETTE["wood_dark"])
    draw.rectangle((2, 10, TILE - 3, TILE - 4), fill=PALETTE["wood_mid"])
    draw.rectangle((1, 4, TILE - 2, 12), fill=PALETTE["teal"])
    draw.line((1, 8, TILE - 2, 8), fill=PALETTE["teal_dark"], width=1)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), outline=PALETTE["outline"], width=1)
    return img


def draw_fridge() -> Image.Image:
    img = new_tile()
    draw = ImageDraw.Draw(img)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), fill=PALETTE["wood_dark"])
    draw.rectangle((5, 2, TILE - 6, TILE - 3), fill=PALETTE["fridge_body"])
    draw.rectangle((5, 2, TILE - 6, 14), fill=PALETTE["fridge_dark"])
    draw.line((5, 14, TILE - 6, 14), fill=PALETTE["outline"], width=1)
    draw.rectangle((TILE - 9, 8, TILE - 7, 20), fill=PALETTE["fridge_handle"])
    draw.rectangle((5, 16, TILE - 6, TILE - 3), fill=PALETTE["fridge_body"])
    draw.rectangle((0, 0, TILE - 1, TILE - 1), outline=PALETTE["outline"], width=1)
    return img


def draw_seed_chest() -> Image.Image:
    img = new_tile()
    draw = ImageDraw.Draw(img)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), fill=PALETTE["wood_dark"])
    draw.rectangle((4, 8, TILE - 5, TILE - 4), fill=PALETTE["chest"])
    draw.rectangle((4, 8, TILE - 5, 12), fill=PALETTE["chest_dark"])
    draw.rectangle((4, 14, TILE - 5, 16), fill=PALETTE["chest_dark"])
    draw.rectangle((14, 12, 18, 16), fill=PALETTE["door_knob"])
    draw.rectangle((0, 0, TILE - 1, TILE - 1), outline=PALETTE["outline"], width=1)
    return img


def draw_sand_floor(variant: int = 0) -> Image.Image:
    img = new_tile()
    draw = ImageDraw.Draw(img)
    base = PALETTE["sand"] if variant == 0 else (176, 150, 108)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), fill=base)
    pebbles = [(6, 8), (20, 14), (14, 22), (24, 6)] if variant == 0 else [(10, 12), (22, 20), (8, 24)]
    for px, py in pebbles:
        draw.point((px, py), fill=PALETTE["sand_pebble"])
        draw.point((px + 1, py), fill=PALETTE["sand_dark"])
    draw.rectangle((0, 0, TILE - 1, TILE - 1), outline=PALETTE["outline"], width=1)
    return img


def draw_fence() -> Image.Image:
    img = new_tile()
    draw = ImageDraw.Draw(img)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), fill=PALETTE["sand"])
    for x in range(2, TILE, 8):
        draw.rectangle((x, 4, x + 3, TILE - 5), fill=PALETTE["fence"])
    draw.rectangle((0, 10, TILE - 1, 14), fill=PALETTE["fence_dark"])
    draw.rectangle((0, 20, TILE - 1, 24), fill=PALETTE["fence"])
    draw.rectangle((0, 0, TILE - 1, TILE - 1), outline=PALETTE["outline"], width=1)
    return img


def draw_gate() -> Image.Image:
    img = new_tile()
    draw = ImageDraw.Draw(img)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), fill=PALETTE["sand"])
    draw.rectangle((10, 2, 13, TILE - 3), fill=PALETTE["gate"])
    draw.rectangle((18, 2, 21, TILE - 3), fill=PALETTE["gate"])
    draw.rectangle((8, 4, TILE - 9, 8), fill=PALETTE["fence_dark"])
    draw.rectangle((8, TILE - 9, TILE - 9, TILE - 5), fill=PALETTE["fence_dark"])
    draw.rectangle((0, 0, TILE - 1, TILE - 1), outline=PALETTE["outline"], width=1)
    return img


def draw_shelter() -> Image.Image:
    img = new_tile()
    draw = ImageDraw.Draw(img)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), fill=PALETTE["sand"])
    draw.polygon([(0, 20), (TILE // 2, 4), (TILE - 1, 20), (TILE - 1, TILE - 1), (0, TILE - 1)], fill=PALETTE["shelter"])
    draw.polygon([(2, 20), (TILE // 2, 8), (TILE - 3, 20)], fill=PALETTE["shelter_dark"])
    draw.rectangle((6, 18, TILE - 7, TILE - 4), fill=PALETTE["shelter_dark"])
    draw.rectangle((0, 0, TILE - 1, TILE - 1), outline=PALETTE["outline"], width=1)
    return img


def draw_feeding_dish() -> Image.Image:
    img = new_tile()
    draw = ImageDraw.Draw(img)
    draw.rectangle((0, 0, TILE - 1, TILE - 1), fill=PALETTE["sand"])
    draw.ellipse((8, 14, TILE - 9, TILE - 7), fill=PALETTE["dish"])
    draw.ellipse((11, 17, TILE - 12, TILE - 10), fill=PALETTE["dish_inner"])
    draw.ellipse((13, 19, 19, 23), fill=PALETTE["food_green"])
    draw.rectangle((0, 0, TILE - 1, TILE - 1), outline=PALETTE["outline"], width=1)
    return img


TILE_DEFS: list[tuple[str, Callable[[], Image.Image], bool]] = [
    ("wood_floor.png", lambda: draw_wood_floor(0), False),
    ("wood_floor_alt.png", lambda: draw_wood_floor(1), False),
    ("wall.png", draw_wall, True),
    ("wood_door.png", draw_wood_door, False),
    ("prep_table.png", draw_prep_table, True),
    ("fridge.png", draw_fridge, False),
    ("seed_chest.png", draw_seed_chest, False),
    ("sand_floor.png", lambda: draw_sand_floor(0), False),
    ("sand_floor_alt.png", lambda: draw_sand_floor(1), False),
    ("fence.png", draw_fence, True),
    ("gate.png", draw_gate, False),
    ("shelter.png", draw_shelter, True),
    ("feeding_dish.png", draw_feeding_dish, False),
]

# Tile indices (0-based in tileset)
WOOD_FLOOR = 0
WOOD_FLOOR_ALT = 1
WALL = 2
WOOD_DOOR = 3
PREP_TABLE = 4
FRIDGE = 5
SEED_CHEST = 6
SAND_FLOOR = 7
SAND_FLOOR_ALT = 8
FENCE = 9
GATE = 10
SHELTER = 11
FEEDING_DISH = 12

FIRST_GID = 1


def gid(tile_id: int) -> int:
    return tile_id + FIRST_GID


def emit_tsx() -> None:
    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        '<tileset version="1.10" tiledversion="1.11.2" name="interiors" '
        f'tilewidth="{TILE}" tileheight="{TILE}" tilecount="{len(TILE_DEFS)}" columns="0">',
        ' <grid orientation="orthogonal" width="1" height="1"/>',
    ]
    for idx, (filename, _, has_obstacle) in enumerate(TILE_DEFS):
        lines.append(f' <tile id="{idx}">')
        lines.append(f'  <image source="tiles/{filename}" width="{TILE}" height="{TILE}"/>')
        if has_obstacle:
            lines.append(OBSTACLE_PROP)
        lines.append(" </tile>")
    lines.append("</tileset>")
    path = OUT_DIR / "interiors.tsx"
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"  wrote {path.relative_to(ROOT)}")


def make_grid(width: int, height: int) -> tuple[list[list[int | None]], list[list[int | None]]]:
    floor: list[list[int | None]] = [[None] * width for _ in range(height)]
    objects: list[list[int | None]] = [[None] * width for _ in range(height)]
    return floor, objects


def nutrition_house_layers() -> tuple[list[list[int | None]], list[list[int | None]]]:
    w, h = 11, 11
    floor, objects = make_grid(w, h)

    for x in range(1, 10):
        for y in range(1, 10):
            floor[y][x] = WOOD_FLOOR if (x + y) % 2 == 0 else WOOD_FLOOR_ALT

    for x in range(w):
        for y in range(h):
            on_perimeter = x == 0 or x == w - 1 or y == 0 or y == h - 1
            if not on_perimeter:
                continue
            if y == 0 and x == 5:
                objects[y][x] = WOOD_DOOR
            else:
                objects[y][x] = WALL

    for x in range(4, 7):
        objects[7][x] = PREP_TABLE

    objects[8][2] = FRIDGE
    objects[5][2] = SEED_CHEST

    return floor, objects


def push_pop_enclosure_layers() -> tuple[list[list[int | None]], list[list[int | None]]]:
    w, h = 13, 13
    floor, objects = make_grid(w, h)

    for x in range(1, 12):
        for y in range(1, 12):
            floor[y][x] = SAND_FLOOR if (x + y) % 2 == 0 else SAND_FLOOR_ALT

    for x in range(w):
        for y in range(h):
            on_perimeter = x == 0 or x == w - 1 or y == 0 or y == h - 1
            if not on_perimeter:
                continue
            if y == 0 and x == 6:
                objects[y][x] = GATE
            else:
                objects[y][x] = FENCE

    for x in range(3, 5):
        for y in range(9, 11):
            objects[y][x] = SHELTER

    objects[6][8] = FEEDING_DISH

    return floor, objects


def grid_to_csv(layer: list[list[int | None]], width: int, height: int) -> str:
    """Convert game-coordinate grid (y=0 bottom) to Tiled CSV (single line)."""
    cells: list[str] = []
    for game_y in range(height - 1, -1, -1):
        for x in range(width):
            tile_id = layer[game_y][x]
            cells.append(str(gid(tile_id)) if tile_id is not None else "0")
    return ",".join(cells)


def emit_tmx(
    filename: str,
    width: int,
    height: int,
    floor: list[list[int | None]],
    objects: list[list[int | None]],
) -> None:
    floor_csv = grid_to_csv(floor, width, height)
    objects_csv = grid_to_csv(objects, width, height)
    content = f"""<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" tiledversion="1.11.2" orientation="orthogonal" renderorder="right-down" width="{width}" height="{height}" tilewidth="{TILE}" tileheight="{TILE}" infinite="0" nextlayerid="3" nextobjectid="1">
 <tileset firstgid="{FIRST_GID}" source="interiors.tsx"/>
 <layer id="1" name="Floor" width="{width}" height="{height}">
  <data encoding="csv">
{floor_csv}
  </data>
 </layer>
 <layer id="2" name="Objects" width="{width}" height="{height}">
  <data encoding="csv">
{objects_csv}
  </data>
 </layer>
</map>
"""
    path = OUT_DIR / filename
    path.write_text(content, encoding="utf-8")
    print(f"  wrote {path.relative_to(ROOT)}")


def main() -> None:
    print("Generating interior tiles...")
    TILES_DIR.mkdir(parents=True, exist_ok=True)

    for filename, draw_fn, _ in TILE_DEFS:
        save_tile(draw_fn(), filename)

    print("Generating tileset...")
    emit_tsx()

    print("Generating maps...")
    nh_floor, nh_objects = nutrition_house_layers()
    emit_tmx("nutrition_house_interior.tmx", 11, 11, nh_floor, nh_objects)

    pp_floor, pp_objects = push_pop_enclosure_layers()
    emit_tmx("push_pop_enclosure_interior.tmx", 13, 13, pp_floor, pp_objects)

    print("Done.")


if __name__ == "__main__":
    main()
