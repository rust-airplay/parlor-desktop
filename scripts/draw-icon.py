
"""Parlor's icon: a P whose counter is a play triangle, in one flat blue.

Drawn at four times the final size and downsampled, so the small sizes get smooth edges.
The application's resources embed assets/icon.ico; this script is the only source of it.
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
ASSETS.mkdir(exist_ok=True)

SIZE = 1024
S = 4
C = SIZE * S

BLUE = (37, 99, 235, 255)
WHITE = (247, 245, 240, 255)


def rounded(mask, box, radius, fill=255):
    ImageDraw.Draw(mask).rounded_rectangle(
        tuple(v * S for v in box), radius=radius * S, fill=fill
    )


def main() -> None:
    tile = Image.new("L", (C, C), 0)
    ImageDraw.Draw(tile).rounded_rectangle(
        (0, 0, C - 1, C - 1), radius=224 * S, fill=255
    )

    letter = Image.new("L", (C, C), 0)
    rounded(letter, (338, 182, 486, 842), 68)
    rounded(letter, (338, 182, 778, 556), 187)

    # The counter is the play triangle: the bowl is hollow where it matters.
    hole = Image.new("L", (C, C), 0)
    ImageDraw.Draw(hole).polygon(
        [(532 * S, 472 * S), (688 * S, 472 * S), (610 * S, 312 * S)], fill=255
    )
    letter = Image.composite(Image.new("L", (C, C), 0), letter, hole)

    canvas = Image.new("RGBA", (C, C), (0, 0, 0, 0))
    blue = Image.new("RGBA", (C, C), BLUE)
    blue.putalpha(tile)
    canvas = Image.alpha_composite(canvas, blue)
    mark = Image.new("RGBA", (C, C), WHITE)
    mark.putalpha(letter)
    canvas = Image.alpha_composite(canvas, mark)

    master = canvas.resize((SIZE, SIZE), Image.LANCZOS)
    master.save(ASSETS / "icon.png")
    sizes = [256, 128, 64, 48, 32, 16]
    for size in sizes:
        master.resize((size, size), Image.LANCZOS).save(ASSETS / f"icon-{size}.png")
    master.save(ASSETS / "icon.ico", sizes=[(size, size) for size in sizes])

    sheet = Image.new("RGB", (256 + 16 * 8 + 80, 286), (120, 120, 124))
    sheet.paste(master.resize((256, 256), Image.LANCZOS), (10, 10))
    zoom = master.resize((16, 16), Image.LANCZOS).resize((128, 128), Image.NEAREST)
    sheet.paste(zoom, (286, 10))
    sheet.paste(master.resize((32, 32), Image.LANCZOS).resize((64, 64), Image.NEAREST), (286, 150))
    sheet.save(ROOT / "target" / "icon-preview.png")


if __name__ == "__main__":
    main()
