
"""Purpose marks, flat, second pass.

A screen and a signal arriving at it. No gradient, no shadow: one tile colour, one mark
colour, and the mark carries the meaning.
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "purpose"
OUT.mkdir(exist_ok=True)

SIZE = 1024
S = 4
C = SIZE * S

INK = (17, 17, 20, 255)
WHITE = (255, 255, 255, 255)
PAPER = (243, 243, 240, 255)


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
    return paint(shape, colour)


def frame(stroke=76, box=(232, 190, 792, 600), radius=56):
    outer = mask()
    rounded(outer, box, radius)
    inner = mask()
    rounded(inner, (box[0] + stroke, box[1] + stroke, box[2] - stroke, box[3] - stroke), max(radius - stroke, 8))
    return Image.composite(mask(), outer, inner)


def signal(centre, radii, width=26, dot=34):
    """A dot with waves opening to the upper right."""
    shape = mask()
    draw = ImageDraw.Draw(shape)
    draw.ellipse(
        ((centre[0] - dot) * S, (centre[1] - dot) * S, (centre[0] + dot) * S, (centre[1] + dot) * S),
        fill=255,
    )
    for radius in radii:
        draw.arc(
            (
                (centre[0] - radius) * S,
                (centre[1] - radius) * S,
                (centre[0] + radius) * S,
                (centre[1] + radius) * S,
            ),
            270,
            360,
            fill=255,
            width=width * S,
        )
    return shape


def finish(canvas, name):
    master = canvas.resize((SIZE, SIZE), Image.LANCZOS)
    master.save(OUT / f"{name}.png")
    return master


def cast_outline(tile_colour, mark_colour):
    mark = frame()
    centre = (330, 500)
    mark = Image.composite(Image.new("L", (C, C), 255), mark, signal(centre, (96, 162, 228)))
    return finish(Image.alpha_composite(tile(tile_colour), paint(mark, mark_colour)), "cast")


def cast_solid(tile_colour, name, mark_colour=WHITE):
    """A solid screen with the signal cut out of it: the boldest of the set."""
    screen = mask()
    rounded(screen, (232, 190, 792, 600), 56)
    cut = signal((366, 500), (96, 162, 228))
    screen = Image.composite(mask(), screen, cut)
    return finish(Image.alpha_composite(tile(tile_colour), paint(screen, mark_colour)), name)


def cast_paper():
    return cast_outline(PAPER, INK)


def main():
    # cast() writes the outline drafts; call it for both colourways.
    masters = [
        cast_outline(INK, WHITE),
        cast_paper(),
        cast_solid(INK, "cast-solid"),
        cast_solid((37, 99, 235, 255), "cast-solid-blue"),
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