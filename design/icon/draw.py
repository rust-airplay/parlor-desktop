
"""Drafts for Parlor's icon, third round.

None of these is wired into the application. Drawn at four times the final size and
downsampled, with the light, the shadows and the glow part of the drawing.
"""

from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "drafts"
OUT.mkdir(exist_ok=True)

SIZE = 1024
S = 4
C = SIZE * S


def linear(start, end):
    yy, xx = np.mgrid[0:C, 0:C]
    t = (xx + yy) / (2 * (C - 1))
    array = np.stack([start[i] + (end[i] - start[i]) * t for i in range(3)], axis=-1)
    return Image.fromarray(array.astype(np.uint8), "RGB").convert("RGBA")


def radial(center, radius, colour, strength, power=2.0):
    yy, xx = np.mgrid[0:C, 0:C]
    distance = np.sqrt((xx - center[0]) ** 2 + (yy - center[1]) ** 2) / radius
    alpha = np.clip(1.0 - distance, 0.0, 1.0) ** power * strength
    array = np.zeros((C, C, 4), dtype=np.uint8)
    array[..., 0], array[..., 1], array[..., 2] = colour
    array[..., 3] = (alpha * 255).astype(np.uint8)
    return Image.fromarray(array, "RGBA")


def mask():
    return Image.new("L", (C, C), 0)


def rounded(mask_image, box, radius, fill=255):
    ImageDraw.Draw(mask_image).rounded_rectangle(
        tuple(v * S for v in box), radius=radius * S, fill=fill
    )


def shadow(shape, offset=(0, 24), blur=56, alpha=130):
    layer = Image.new("RGBA", (C, C), (5, 8, 14, alpha))
    layer.putalpha(shape)
    layer = layer.filter(ImageFilter.GaussianBlur(blur * S / 4))
    moved = Image.new("RGBA", (C, C), (0, 0, 0, 0))
    moved.paste(layer, (offset[0] * S, offset[1] * S), layer)
    return moved


def paint(shape, colour):
    layer = Image.new("RGBA", (C, C), colour)
    layer.putalpha(shape)
    return layer


def gradient_shape(shape, start, end):
    fill = linear(start, end)
    fill.putalpha(shape)
    return fill


def tile(start, end, radius=224):
    shape = mask()
    rounded(shape, (0, 0, SIZE, SIZE), radius)
    canvas = paint(shape, (0, 0, 0, 255))
    canvas = Image.alpha_composite(canvas, gradient_shape(shape, start, end))
    top_light = Image.composite(
        radial((C * 0.3, C * 0.14), C * 0.9, (255, 255, 255), 0.10),
        Image.new("RGBA", (C, C)),
        shape,
    )
    return Image.alpha_composite(canvas, top_light)


def finish(canvas, name):
    master = canvas.resize((SIZE, SIZE), Image.LANCZOS)
    master.save(OUT / f"{name}.png")
    return master


def draft_screen():
    """A dark room with a screen that is lit."""
    canvas = tile((12, 20, 36), (20, 27, 46))
    canvas = Image.alpha_composite(canvas, radial((C * 0.5, C * 0.45), C * 0.5, (34, 211, 238), 0.26))

    body = mask()
    rounded(body, (238, 202, 786, 586), 68)
    canvas = Image.alpha_composite(canvas, shadow(body, offset=(0, 34), blur=70, alpha=150))
    canvas = Image.alpha_composite(canvas, paint(body, (8, 13, 22, 255)))

    inner = mask()
    rounded(inner, (258, 222, 766, 566), 54)
    canvas = Image.alpha_composite(canvas, gradient_shape(inner, (6, 182, 212), (79, 70, 229)))
    shine = radial((C * 0.3, C * 0.26), C * 0.34, (255, 255, 255), 0.32)
    canvas = Image.alpha_composite(canvas, Image.composite(shine, Image.new("RGBA", (C, C)), inner))

    # A thin bright edge along the top of the bezel: glass, not plastic.
    edge = mask()
    rounded(edge, (238, 202, 786, 586), 68)
    rounded(edge, (246, 210, 778, 578), 62, fill=0)
    canvas = Image.alpha_composite(canvas, paint(edge, (255, 255, 255, 40)))

    triangle = mask()
    ImageDraw.Draw(triangle).polygon(
        [(436 * S, 542 * S), (588 * S, 542 * S), (512 * S, 444 * S)], fill=255
    )
    canvas = Image.alpha_composite(canvas, paint(triangle, (255, 255, 255, 255)))
    canvas = Image.alpha_composite(canvas, radial((C * 0.5, C * 0.66), C * 0.36, (34, 211, 238), 0.14))
    return finish(canvas, "screen")


def draft_lamp():
    """A pendant lamp, for the room the application is named after."""
    canvas = tile((31, 27, 43), (17, 16, 24))

    cone = mask()
    ImageDraw.Draw(cone).polygon(
        [(356 * S, 590 * S), (668 * S, 590 * S), (816 * S, 1010 * S), (208 * S, 1010 * S)],
        fill=255,
    )
    cone = Image.composite(
        radial((C * 0.5, C * 0.5), C * 0.55, (253, 230, 138), 0.55), Image.new("RGBA", (C, C)), cone
    )
    canvas = Image.alpha_composite(canvas, cone)

    cord = mask()
    ImageDraw.Draw(cord).line([(512 * S, 0), (512 * S, 238 * S)], fill=255, width=9 * S)
    canvas = Image.alpha_composite(canvas, paint(cord, (168, 162, 158, 255)))

    shade = mask()
    ImageDraw.Draw(shade).pieslice((318 * S, 210 * S, 706 * S, 598 * S), 180, 360, fill=255)
    canvas = Image.alpha_composite(canvas, shadow(shade, offset=(0, 20), blur=44, alpha=130))
    canvas = Image.alpha_composite(canvas, gradient_shape(shade, (255, 251, 235), (251, 191, 36)))

    rim = mask()
    ImageDraw.Draw(rim).pieslice((318 * S, 210 * S, 706 * S, 598 * S), 180, 360, fill=255)
    ImageDraw.Draw(rim).pieslice((326 * S, 218 * S, 698 * S, 590 * S), 180, 360, fill=0)
    canvas = Image.alpha_composite(canvas, paint(rim, (255, 255, 255, 110)))

    canvas = Image.alpha_composite(canvas, radial((C * 0.5, C * 0.60), C * 0.24, (255, 251, 235), 0.9))
    bulb = mask()
    ImageDraw.Draw(bulb).ellipse((474 * S, 562 * S, 550 * S, 638 * S), fill=255)
    canvas = Image.alpha_composite(canvas, paint(bulb, (255, 255, 255, 255)))
    return finish(canvas, "lamp")


def draft_television():
    """A warm set in a dark parlour."""
    canvas = tile((39, 29, 44), (24, 20, 30))
    canvas = Image.alpha_composite(canvas, radial((C * 0.5, C * 0.5), C * 0.5, (251, 191, 36), 0.10))

    # Two legs first, so the body sits on them.
    legs = mask()
    ImageDraw.Draw(legs).line(
        [(318 * S, 694 * S), (284 * S, 816 * S)], fill=255, width=32 * S
    )
    ImageDraw.Draw(legs).line(
        [(702 * S, 694 * S), (736 * S, 816 * S)], fill=255, width=32 * S
    )
    canvas = Image.alpha_composite(canvas, paint(legs, (120, 53, 15, 255)))

    body = mask()
    rounded(body, (198, 258, 826, 704), 78)
    canvas = Image.alpha_composite(canvas, shadow(body, offset=(0, 30), blur=60, alpha=150))
    canvas = Image.alpha_composite(canvas, gradient_shape(body, (255, 251, 235), (253, 230, 138)))

    screen = mask()
    rounded(screen, (252, 306, 634, 656), 36)
    canvas = Image.alpha_composite(canvas, gradient_shape(screen, (9, 32, 52), (7, 74, 104)))
    canvas = Image.alpha_composite(
        canvas, Image.composite(radial((C * 0.44, C * 0.52), C * 0.30, (34, 211, 238), 0.55), Image.new("RGBA", (C, C)), screen)
    )
    triangle = mask()
    ImageDraw.Draw(triangle).polygon(
        [(386 * S, 606 * S), (548 * S, 606 * S), (467 * S, 502 * S)], fill=255
    )
    canvas = Image.alpha_composite(canvas, paint(triangle, (255, 255, 255, 255)))

    # Two dials on the right.
    for x, y, radius in ((706, 398, 42), (706, 520, 32)):
        dial = mask()
        ImageDraw.Draw(dial).ellipse(
            ((x - radius) * S, (y - radius) * S, (x + radius) * S, (y + radius) * S), fill=255
        )
        canvas = Image.alpha_composite(canvas, paint(dial, (180, 83, 9, 255)))
        hole = mask()
        ImageDraw.Draw(hole).ellipse(
            ((x - radius // 2) * S, (y - radius // 2) * S, (x + radius // 2) * S, (y + radius // 2) * S),
            fill=255,
        )
        canvas = Image.alpha_composite(canvas, paint(hole, (255, 251, 235, 255)))
    return finish(canvas, "television")


def draft_neon():
    """A screen drawn in light: dark room, cool edge, no object at all."""
    canvas = tile((9, 9, 13), (20, 20, 28))
    canvas = Image.alpha_composite(canvas, radial((C * 0.72, C * 0.24), C * 0.55, (109, 40, 217), 0.22))

    frame = mask()
    rounded(frame, (244, 208, 780, 556), 60)
    inset = mask()
    rounded(inset, (280, 244, 744, 520), 34)
    frame = Image.composite(mask(), frame, inset)
    canvas = Image.alpha_composite(canvas, paint(frame, (245, 245, 245, 245)))

    triangle = mask()
    ImageDraw.Draw(triangle).polygon(
        [(418 * S, 774 * S), (606 * S, 774 * S), (512 * S, 562 * S)], fill=255
    )
    glow = Image.new("RGBA", (C, C), (0, 0, 0, 0))
    glow.paste(gradient_shape(triangle, (34, 211, 238), (139, 92, 246)), (0, 0))
    glow = glow.filter(ImageFilter.GaussianBlur(38 * S / 4))
    canvas = Image.alpha_composite(canvas, glow)
    canvas = Image.alpha_composite(canvas, gradient_shape(triangle, (34, 211, 238), (139, 92, 246)))
    return finish(canvas, "neon")


def draft_monogram():
    """A P whose counter is a play mark."""
    canvas = tile((219, 39, 119), (234, 88, 12))
    letter = mask()
    rounded(letter, (344, 190, 492, 834), 74)
    rounded(letter, (344, 190, 764, 566), 190)
    hole = mask()
    ImageDraw.Draw(hole).polygon(
        [(520 * S, 480 * S), (666 * S, 480 * S), (593 * S, 330 * S)], fill=255
    )
    letter = Image.composite(mask(), letter, hole)
    canvas = Image.alpha_composite(canvas, shadow(letter, offset=(0, 16), blur=30, alpha=90))
    canvas = Image.alpha_composite(canvas, gradient_shape(letter, (255, 255, 255), (254, 226, 226)))
    return finish(canvas, "monogram")


def main():
    masters = [draft_television(), draft_lamp(), draft_screen(), draft_neon()]
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