
"""Three focused drafts. Flat colour only; the mark is cut out of the shape, not drawn on
top of it, which is what gives these their weight without any gradient or shadow.
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "final"
OUT.mkdir(exist_ok=True)

SIZE = 1024
S = 4
C = SIZE * S

INK = (17, 17, 20, 255)
INDIGO = (30, 27, 75, 255)
PAPER = (247, 245, 240, 255)
TEAL = (19, 78, 74, 255)


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


def finish(canvas, name):
    master = canvas.resize((SIZE, SIZE), Image.LANCZOS)
    master.save(OUT / f"{name}.png")
    return master


def cast_inverse():
    """A solid screen with the signal punched out of it."""
    screen = mask()
    rounded(screen, (176, 184, 848, 640), 96)

    signal = mask()
    draw = ImageDraw.Draw(signal)
    centre = (332, 528)
    draw.ellipse(
        ((centre[0] - 62) * S, (centre[1] - 62) * S, (centre[0] + 62) * S, (centre[1] + 62) * S),
        fill=255,
    )
    for radius in (164, 300):
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
            width=48 * S,
        )
    cut = Image.composite(mask(), screen, signal)
    return finish(Image.alpha_composite(tile(INDIGO), paint(cut, PAPER)), "cast-inverse")


def p_play():
    """A P whose counter is the play triangle."""
    letter = mask()
    rounded(letter, (338, 182, 486, 842), 68)
    rounded(letter, (338, 182, 778, 556), 187)
    hole = mask()
    ImageDraw.Draw(hole).polygon(
        [(532 * S, 472 * S), (688 * S, 472 * S), (610 * S, 312 * S)], fill=255
    )
    letter = Image.composite(mask(), letter, hole)
    return finish(Image.alpha_composite(tile(INDIGO), paint(letter, PAPER)), "p-play")


def tri_screen():
    """A screen with the AirPlay triangle punched out, nothing else."""
    screen = mask()
    rounded(screen, (176, 196, 848, 700), 100)
    triangle = mask()
    ImageDraw.Draw(triangle).polygon(
        [(392 * S, 620 * S), (632 * S, 620 * S), (512 * S, 440 * S)], fill=255
    )
    cut = Image.composite(mask(), screen, triangle)
    return finish(Image.alpha_composite(tile(TEAL), paint(cut, PAPER)), "tri-screen")


def main():
    masters = [cast_inverse(), p_play(), tri_screen()]
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

    # What sixteen pixels actually look like, magnified eight times without smoothing.
    zoom = Image.new("RGB", (len(masters) * (128 + 18) + 18, 146), (120, 120, 124))
    x = 18
    for master in masters:
        small = master.resize((16, 16), Image.LANCZOS).resize((128, 128), Image.NEAREST)
        zoom.paste(small, (x, 9))
        x += 128 + 18
    zoom.save(OUT / "sixteen.png")


if __name__ == "__main__":
    main()