
"""Draws Parlor's icon.

The mark is a monitor with a bezel and an AirPlay triangle on its screen: the application
receives a device's screen, so the icon is the screen it plays on.

Run with the bundled Python; the files land in assets/.
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
ASSETS.mkdir(exist_ok=True)

# Drawn at four times the final size, then downsampled: every edge lands on a whole pixel.
SIZE = 1024
SCALE = 4
C = SIZE * SCALE

# The tile's gradient: cyan in the top left, indigo in the bottom right.
START = (34, 211, 238)
END = (67, 56, 202)


def gradient(size: int, start, end) -> Image.Image:
    """A diagonal gradient, one row at a time."""
    image = Image.new("RGB", (size, size))
    pixels = image.load()
    for y in range(size):
        for x in range(size):
            # 0 at the top left corner, 1 at the bottom right.
            t = (x + y) / (2 * (size - 1))
            pixels[x, y] = tuple(
                round(start[channel] + (end[channel] - start[channel]) * t)
                for channel in range(3)
            )
    return image


def rounded_mask(size: int, box, radius: int, fill: int = 255) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(box, radius=radius, fill=fill)
    return mask


def main() -> None:
    tile_mask = rounded_mask(C, (0, 0, C - 1, C - 1), radius=220 * SCALE)
    tile = gradient(C, START, END).convert("RGBA")
    tile.putalpha(tile_mask)

    # The white bezel: a rounded rectangle with the screen cut out of it.
    bezel = Image.new("L", (C, C), 0)
    draw = ImageDraw.Draw(bezel)
    draw.rounded_rectangle(
        (230 * SCALE, 190 * SCALE, 794 * SCALE, 620 * SCALE),
        radius=64 * SCALE,
        fill=255,
    )
    screen = rounded_mask(
        C,
        (270 * SCALE, 230 * SCALE, 754 * SCALE, 580 * SCALE),
        radius=28 * SCALE,
        fill=255,
    )
    bezel = Image.composite(Image.new("L", (C, C), 0), bezel, screen)

    white = Image.new("RGBA", (C, C), (255, 255, 255, 255))
    white.putalpha(bezel)

    # The AirPlay triangle sits on the screen, tip up.
    triangle = Image.new("L", (C, C), 0)
    ImageDraw.Draw(triangle).polygon(
        [
            (390 * SCALE, 580 * SCALE),
            (634 * SCALE, 580 * SCALE),
            (512 * SCALE, 392 * SCALE),
        ],
        fill=255,
    )
    triangle = Image.composite(triangle, Image.new("L", (C, C), 0), screen)
    white_triangle = Image.new("RGBA", (C, C), (255, 255, 255, 255))
    white_triangle.putalpha(triangle)

    # The stand: a neck and a foot, both white.
    stand = Image.new("L", (C, C), 0)
    stand_draw = ImageDraw.Draw(stand)
    stand_draw.rounded_rectangle(
        (474 * SCALE, 620 * SCALE, 550 * SCALE, 700 * SCALE),
        radius=10 * SCALE,
        fill=255,
    )
    stand_draw.rounded_rectangle(
        (386 * SCALE, 700 * SCALE, 638 * SCALE, 732 * SCALE),
        radius=16 * SCALE,
        fill=255,
    )
    white_stand = Image.new("RGBA", (C, C), (255, 255, 255, 255))
    white_stand.putalpha(stand)

    icon = Image.alpha_composite(tile, white)
    icon = Image.alpha_composite(icon, white_triangle)
    icon = Image.alpha_composite(icon, white_stand)

    master = icon.resize((SIZE, SIZE), Image.LANCZOS)
    master.save(ASSETS / "icon.png")

    sizes = [256, 128, 64, 48, 32, 16]
    for size in sizes:
        master.resize((size, size), Image.LANCZOS).save(ASSETS / f"icon-{size}.png")
    master.save(ASSETS / "icon.ico", sizes=[(size, size) for size in sizes])

    # A sheet for judging the small sizes the way they are actually seen.
    sheet = Image.new("RGBA", (256 + 128 + 64 + 48 + 32 + 16 + 70, 256), (0, 0, 0, 0))
    x = 5
    for size in [256, 128, 64, 48, 32, 16]:
        small = master.resize((size, size), Image.LANCZOS)
        sheet.alpha_composite(small, (x, (256 - size) // 2))
        x += size + 10
    sheet.save(ROOT / "target" / "icon-sheet.png")


if __name__ == "__main__":
    main()
