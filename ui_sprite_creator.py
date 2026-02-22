import json
from PIL import Image
import os


with open("assets/atlas.json", "r") as f:
    data = json.load(f)


# Open assets/atlas.png image
atlas_img = Image.open("assets/atlas.png").convert("RGBA")
tile_img = Image.open("assets/tile_backgrounds.png").convert("RGBA")


def process_atlas(out_name: str, in_name_override: str | None = None, index: int = 0):
    in_name = in_name_override or out_name
    frames = [
        f["frame"]
        for f in
        data["frames"]
        if all(
            s in f["filename"]
            for s in [in_name, f" {index}.aseprite"]
        )
    ]
    if len(frames) != 1:
        breakpoint()

    frame = frames[0]

    x, y, w, h = frame["x"], frame["y"], frame["w"], frame["h"]

    # Crop the sprite from the atlas
    cropped = atlas_img.crop((x, y, x + w, y + h))

    out = Image.new("RGBA", (24, 24), (0, 0, 0, 0))
    ox = (24 - w) // 2
    oy = (24 - h) // 2
    out.paste(cropped, (ox, oy))

    # Save
    out_path = os.path.join("assets", "ui_sprites", f"{out_name}.png")
    out.save(out_path)
    print(f"Saved {out_name} -> {out_path}")


def process_tile_background(name: str, index: int):
    cropped = tile_img.crop((64 * index, 0, 64 * index + 16, 16))
    out_path = os.path.join("assets", "ui_sprites", f"{name}.png")
    cropped.save(out_path)
    print(f"Saved {name}")


if __name__ == "__main__":
    process_atlas("bat")
    process_atlas("burrower")
    process_atlas("slime")
    process_atlas("worm")
    process_tile_background("stone", 0)
    process_tile_background("wood", 1)
    process_tile_background("not_part_of", 2)
    process_tile_background("clear", 4)
    process_atlas("ladder", "tiles", 0)
    process_atlas("platform", "tiles", 2)
    process_atlas("start_door", "tiles", 7)
