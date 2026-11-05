
"""Flat drafts for Parlor's icon.

No gradients, no shadows, no glow: one colour for the tile, one for the mark. Drawn at four
times the final size and downsampled, which is only how the edges are smoothed.
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "flat"
OUT.mkdir(exist_ok=True)

SIZE = 1024
S = 4
C = SIZE * S

INK = (16, 16, 20, 255)
PAPER = (242, 242, 239, 255)
BLUE = (37, 99, 235, 255)
WHITE = (255, 255, 255, 255)


def mask():
    return Image.new("L", (C, C), 0)


def rounded(mask_image, box, radius, fill=255):
    ImageDraw.Draw(mask_image).rounded_rectangle(
        tuple(v * S for v in box), radius=radius * S, fill=fill
    )


def paint(shape, colour):
    layer = Image.new("RGBA", (C, C), colour)
    layer.putalpha(shape)
    return layer


def tile(colour, radius=224):
    shape = mask()
    rounded(shape, (0, 0, SIZE, SIZE), radius)
    canvas = Image.new("RGBA", (C, C), (0, 0, 0, 0))
    return Image.alpha_composite(canvas, paint(shape, colour))


def airplay(colour, stroke=84):
    """A screen outline with the triangle under it, drawn as one weight."""
    frame = mask()
    rounded(frame, (240, 214, 784, 598), 60)
    window = mask()
    rounded(
        window,
        (240 + stroke, 214 + stroke, 784 - stroke, 598 - stroke),
        max(60 - stroke, 8),
    )
    frame = Image.composite(mask(), frame, window)

    # The triangle is a third of the screen wide: the AirPlay proportion, not a stand.
    triangle = mask()
    ImageDraw.Draw(triangle).polygon(
        [(432 * S, 790 * S), (592 * S, 790 * S), (512 * S, 600 * S)], fill=255
    )
    mark = Image.composite(Image.new("L", (C, C), 255), frame, triangle)
    return paint(mark, colour)


def monogram(colour):
    """A P built from a stem, a bowl and a round counter."""
    letter = mask()
    rounded(letter, (352, 196, 500, 780), 74)
    rounded(letter, (352, 196, 780, 556), 190)
    hole = mask()
    rounded(hole, (500, 292, 700, 470), 90)
    letter = Image.composite(mask(), letter, hole)
    return paint(letter, colour)


def finish(canvas, name):
    canvas.resize((SIZE, SIZE), Image.LANCZOS).save(OUT / f"{name}.png")
    return canvas.resize((SIZE, SIZE), Image.LANCZOS)


def main():
    masters = [
        finish(Image.alpha_composite(tile(INK), airplay(WHITE)), "ink"),
        finish(Image.alpha_composite(tile(PAPER), airplay(INK)), "paper"),
        finish(Image.alpha_composite(tile(BLUE), airplay(WHITE)), "blue"),
        finish(Image.alpha_composite(tile(INK), monogram(WHITE)), "p"),
    ]
    cell, gap = 256, 18
    width = len(masters) * cell + (len(masters) + 1) * gap
    sheet = Image.new("RGB", (width, cell + 2 * gap), (120, 120, 124))
    x = gap
    for master in masters:
        sheet.paste(master.resize((cell, cell), Image.LANCZOS), (x, gap))
        for i, size in enumerate((32, 16)):
            small = master.resize((size, size), Image.LANCZOS)
            sheet.paste(small, (x + cell - size - 8 - i * 30, gap + cell - size - 8), small)
        x += cell + gap
    sheet.save(OUT / "sheet.png")


if __name__ == "__main__":
    main()