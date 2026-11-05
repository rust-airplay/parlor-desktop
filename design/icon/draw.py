
"""Drafts for Parlor's icon.

None of these is wired into the application. Each is drawn at four times the final size and
downsampled, which is what keeps sixteen pixels legible.
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "drafts"
OUT.mkdir(exist_ok=True)

SIZE = 1024
S = 4
C = SIZE * S


def gradient(start, end):
    image = Image.new("RGB", (C, C))
    pixels = image.load()
    for y in range(C):
        for x in range(C):
            t = (x + y) / (2 * (C - 1))
            pixels[x, y] = tuple(round(start[i] + (end[i] - start[i]) * t) for i in range(3))
    return image.convert("RGBA")


def rounded_tile(start, end, radius=224):
    base = gradient(start, end)
    mask = Image.new("L", (C, C), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, C - 1, C - 1), radius=radius * S, fill=255)
    base.putalpha(mask)
    return base


def white_mask():
    return Image.new("L", (C, C), 0)


def rounded(mask, box, radius, fill=255):
    ImageDraw.Draw(mask).rounded_rectangle(tuple(v * S for v in box), radius=radius * S, fill=fill)


def ring(mask, box, radius, width):
    """A rounded rectangle outline: the outer shape minus the inset shape."""
    outer = white_mask()
    rounded(outer, box, radius)
    inset = (box[0] + width, box[1] + width, box[2] - width, box[3] - width)
    hole = white_mask()
    rounded(hole, inset, max(radius - width, 4))
    return Image.composite(white_mask(), outer, hole)


def polygon(mask, points, fill=255):
    ImageDraw.Draw(mask).polygon([(x * S, y * S) for x, y in points], fill=fill)


def paint(canvas, mask, colour=(255, 255, 255, 255)):
    layer = Image.new("RGBA", (C, C), colour)
    layer.putalpha(mask)
    return Image.alpha_composite(canvas, layer)


def glyph(canvas, mask, start, end):
    """Paints a mask with a gradient instead of a flat colour."""
    fill = gradient(start, end)
    fill.putalpha(mask)
    return Image.alpha_composite(canvas, fill)


def airplay_mark(mask):
    """A screen outline with the AirPlay triangle under it, tip touching the frame."""
    # The pair is centred on the tile, not the frame alone.
    frame = ring(mask, (248, 214, 776, 598), 64, 72)
    ImageDraw.Draw(frame).polygon(
        [(408 * S, 810 * S), (616 * S, 810 * S), (512 * S, 602 * S)], fill=255
    )
    return frame


def cast_mark(mask):
    """A screen with the signal in its lower-left corner."""
    frame = ring(mask, (216, 196, 808, 592), 72, 68)
    draw = ImageDraw.Draw(frame)
    # The waves open towards the top right, out of the corner, so they stay on the screen.
    centre = (330, 500)
    for radius in (92, 164, 236):
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
            width=24 * S,
        )
    draw.ellipse(
        ((centre[0] - 30) * S, (centre[1] - 30) * S, (centre[0] + 30) * S, (centre[1] + 30) * S),
        fill=255,
    )
    return frame


def lamp_mark():
    lamp = white_mask()
    ImageDraw.Draw(lamp).polygon(
        [
            (408 * S, 236 * S),
            (616 * S, 236 * S),
            (692 * S, 536 * S),
            (332 * S, 536 * S),
        ],
        fill=255,
    )
    # A lip on the shade, then a stem and a foot.
    rounded(lamp, (322, 518, 702, 566), 24)
    rounded(lamp, (488, 566, 536, 704), 24)
    rounded(lamp, (376, 704, 648, 748), 22)
    return lamp


def draft(name, tile, mask, gradient_glyph=None):
    canvas = tile
    if gradient_glyph:
        canvas = glyph(canvas, mask, *gradient_glyph)
    else:
        canvas = paint(canvas, mask)
    master = canvas.resize((SIZE, SIZE), Image.LANCZOS)
    master.save(OUT / f"{name}.png")
    return master


def main():
    masters = [
        draft("airplay", rounded_tile((14, 165, 233), (99, 102, 241)), airplay_mark(white_mask())),
        draft(
            "airplay-light",
            rounded_tile((250, 250, 252), (228, 228, 236)),
            airplay_mark(white_mask()),
            gradient_glyph=((14, 165, 233), (109, 40, 217)),
        ),
        draft("cast", rounded_tile((124, 58, 237), (219, 39, 119)), cast_mark(white_mask())),
        draft("lamp", rounded_tile((245, 158, 11), (220, 38, 38)), lamp_mark()),
    ]

    cell, gap = 256, 18
    width = len(masters) * cell + (len(masters) + 1) * gap
    sheet = Image.new("RGB", (width, cell + 2 * gap), (24, 24, 27))
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