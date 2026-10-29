"""Render the motion preview for split-keyboard-mouse-views as SVG frames.

The keyboard comes from g512-control/src/layout.rs; the mouse paths and the
timings are the ones on the Penpot "04 Motion" board. Encode the frames with:

    python3 -I render.py ../../../../../g512-control/src/layout.rs OUT_DIR
    ffmpeg -framerate 30 -i OUT_DIR/f%04d.svg -c:v libvpx-vp9 -b:v 0 -crf 32 motion-preview.webm
"""
import math
import os
import re
import sys

FPS, SECONDS = 30, 12.0
W, H = 1280, 720
GREEN = (0x26, 0xA2, 0x69)
FONT = "Adwaita Sans, Cantarell, Inter, sans-serif"


def keys_from(layout_rs):
    src = open(layout_rs).read()
    consts = {"F": 0, "R1": 6, "R2": 10, "R3": 14, "R4": 18, "R5": 22, "NAV": 61, "PAD": 74}
    ev = lambda e: eval(e, {}, consts)
    keys = []
    for m in re.finditer(r'\bk\((0x[0-9a-f]+), "(?:[^"\\]|\\.)*", "[^"]*", ([^,]+), ([^,]+), (\d+)\)', src):
        keys.append((ev(m.group(2)), ev(m.group(3)), int(m.group(4)), 4, int(m.group(1), 16)))
    for m in re.finditer(r'tall\((0x[0-9a-f]+), "[^"]*", "[^"]*", ([^,]+), ([^)]+)\)', src):
        keys.append((ev(m.group(2)), ev(m.group(3)), 4, 8, int(m.group(1), 16)))
    keys.append((45, 22, 5, 4, 0))  # Fn: no light
    return keys


def rgb(c, a=1.0):
    return f'fill="rgb{c}" fill-opacity="{a:.3f}"'


def ease_out_cubic(p):
    return 1 - (1 - p) ** 3


def ease_in_out_sine(p):
    return -(math.cos(math.pi * p) - 1) / 2


def hue_shift(c, deg):
    # Rotate hue for Cycle (simple YIQ rotation).
    r, g, b = [v / 255 for v in c]
    y = 0.299 * r + 0.587 * g + 0.114 * b
    i = 0.596 * r - 0.274 * g - 0.322 * b
    q = 0.211 * r - 0.523 * g + 0.312 * b
    a = math.radians(deg)
    i, q = i * math.cos(a) - q * math.sin(a), i * math.sin(a) + q * math.cos(a)
    out = (y + 0.956 * i + 0.621 * q, y - 0.272 * i - 0.647 * q, y - 1.106 * i + 1.703 * q)
    return tuple(max(0, min(255, round(v * 255))) for v in out)


def keyboard(keys, t, ox, oy, U=8):
    """Returns (svg, caption). Ripple 0-4.8 s, Wave 4.8-7.8 s, Breathe 7.8-11.8 s."""
    P = 16
    w, h = 90 * U + 2 * P, 26 * U + 2 * P
    out = [f'<rect x="{ox + 2}" y="{oy + 2}" width="{w - 4}" height="{h - 4}" rx="18" fill="none" '
           f'stroke="rgb{GREEN}" stroke-width="3"/>']
    g_centre = (25, 16)
    if t < 4.8:
        p = (t % 2.4) / 2.4
        r = ease_out_cubic(p) * 52
        fade = 1 - max(0.0, (p - 0.6) / 0.4)
        caption = "Ripple · 2.4 s per ring"

        def level(cx, cy):
            d = math.hypot(cx - g_centre[0], (cy - g_centre[1]) * 1.2)
            return fade * max(0.0, 1 - abs(d - r) / 5) if abs(d - r) < 5 else 0
    elif t < 7.8:
        pos = -6 + (t - 4.8) / 3.0 * 102
        caption = "Wave · 3 s sweep"

        def level(cx, cy):
            d = abs(cx - pos)
            return max(0.0, 1 - d / 10)
    else:
        p = ((t - 7.8) % 4.0) / 4.0
        v = 0.15 + 0.85 * ease_in_out_sine(1 - abs(1 - 2 * p))
        caption = "Breathe · 4 s"

        def level(cx, cy):
            return v
    for x, y, kw, kh, usage in keys:
        kx, ky = ox + P + x * U + 2, oy + P + y * U + 2
        ww, hh = kw * U - 4, kh * U - 4
        lv = level(x + kw / 2, y + kh / 2) if usage else 0
        out.append(f'<rect x="{kx + 0.75}" y="{ky + 0.75}" width="{ww - 1.5}" height="{hh - 1.5}" rx="3.5" fill="none" '
                   f'stroke="rgb{GREEN}" stroke-opacity="0.25" stroke-width="1.5"/>')
        if lv > 0.02:
            out.append(f'<rect x="{kx}" y="{ky}" width="{ww}" height="{hh}" rx="3.5" {rgb(GREEN, lv)}/>')
    return "\n".join(out), caption, (w, h)


INK_TOP = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "mouse-art", "g502-top-ink.svg")
_ink = open(INK_TOP).read()
INK_BODY = re.sub(r"<title>[\s\S]*?</title>", "", _ink.split("</defs>", 1)[1].rsplit("</svg>", 1)[0])
CARD = "#1D1D20"
G_CUT = (f'<path d="M172.2 507.6 A15 15 0 1 0 176.6 518.2 L163.2 518.2" fill="none" stroke="{CARD}" '
         f'stroke-width="6.5" stroke-linecap="round" stroke-linejoin="round"/>')
# G9: the lower square behind the wheel, in the ink file's coordinates.
G9 = (211.7, 346.8, 41.6, 52.2, 232.5, 373.0)


def mouse(t, ox, oy, s, led_level, led_colour):
    """The ink top view at scale s. G9: hover 1.0-3.0 s, Apply confirm at 6.0 s."""
    body = (INK_BODY
            .replace('class="cls-1"', f'fill="rgb{led_colour}" fill-opacity="{0.28 * led_level:.3f}"')
            .replace('class="cls-2"', f'fill="rgb{GREEN}"')
            .replace('class="cls-3"', f'fill="rgb{led_colour}" fill-opacity="{led_level:.3f}"'))
    parts = [f'<g transform="translate({ox} {oy}) scale({s})">', body, G_CUT]
    caption = "Mouse · idle"
    fill, ring, ring_o, flash, arrow = 0.0, 0, 0, False, False
    if 1.0 <= t < 3.0:  # hover
        e = t - 1.0
        fill = ease_out_cubic(min(1, e / 0.25))
        if e < 0.6:
            ring, ring_o = 30 + ease_out_cubic(e / 0.6) * 45, 1 - e / 0.6
        arrow = e > 0.15
        caption = "Hover on G9 · fill 250 ms, ring 600 ms"
    elif 3.0 <= t < 3.25:
        fill = 1 - (t - 3.0) / 0.25
        caption = "Hover ends"
    elif 6.0 <= t < 7.85:  # confirm after Apply
        e = t - 6.0
        flash = e < 0.25
        fill = 1.0 if e < 1.6 else max(0.0, 1 - (e - 1.6) / 0.25)
        if e < 0.6:
            ring, ring_o = 30 + ease_out_cubic(e / 0.6) * 50, 1 - e / 0.6
        caption = "Apply → confirm pulse, rest 1 s, fade"
    gx, gy, gw, gh, cx, cy = G9
    if fill > 0.01:
        c = (255, 255, 255) if flash else GREEN
        parts.append(f'<rect x="{gx}" y="{gy}" width="{gw}" height="{gh}" rx="5" {rgb(c, fill)}/>')
    if ring:
        c = (255, 255, 255) if flash else GREEN
        parts.append(f'<circle cx="{cx}" cy="{cy}" r="{ring:.2f}" fill="none" stroke="rgb{c}" stroke-opacity="{ring_o:.3f}" stroke-width="5"/>')
    if arrow:
        a = min(1, (t - 1.15) / 0.2)
        n = 8 * math.sin((t - 1.15) * 2 * math.pi / 0.9)
        parts.append(f'<path d="M{335 + n:.1f} {cy} L{278 + n:.1f} {cy} M{296 + n:.1f} {cy - 18} L{278 + n:.1f} {cy} L{296 + n:.1f} {cy + 18}" '
                     f'fill="none" stroke="rgb{GREEN}" stroke-opacity="{a:.2f}" stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/>')
    parts.append("</g>")
    return "\n".join(parts), caption


def frame(keys, t):
    kb_svg, kb_cap, (kw, kh) = keyboard(keys, t, 64, 238)
    # Mouse LEDs follow the keyboard effect (same timing).
    if t < 4.8:
        p = (t % 2.4) / 2.4
        led = 0.3 + 0.7 * max(0.0, 1 - p / 0.4)
    elif t < 7.8:
        led = 0.3 + 0.7 * max(0.0, 1 - abs((t - 4.8) / 3.0 - 0.95) / 0.25)
    else:
        p = ((t - 7.8) % 4.0) / 4.0
        led = 0.15 + 0.85 * ease_in_out_sine(1 - abs(1 - 2 * p))
    m_svg, m_cap = mouse(t, 940, 128, 0.574, led, GREEN)
    text = lambda x, y, s, size=17, w=400, o=1.0: (f'<text x="{x}" y="{y}" font-family="{FONT}" font-size="{size}" '
                                                    f'font-weight="{w}" fill="#FFFFFF" fill-opacity="{o}">{s}</text>')
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">
<rect width="{W}" height="{H}" fill="#222226"/>
<rect x="32" y="32" width="{W - 64}" height="{H - 140}" rx="16" fill="#1D1D20"/>
{text(64, 92, "G512 Control · motion preview", 22, 700)}
{text(64, 118, "Line art in the live LED colour; timings from the 04 Motion board", 15, 400, 0.55)}
{kb_svg}
{m_svg}
<rect x="32" y="{H - 92}" width="{W - 64}" height="60" rx="12" fill="#2E2E33"/>
{text(64, H - 56, "Keyboard: " + kb_cap, 17, 700)}
{text(700, H - 56, m_cap, 17, 400, 0.75)}
{text(W - 150, H - 56, f"{t:5.2f} s", 15, 400, 0.55)}
</svg>"""


def main():
    keys = keys_from(sys.argv[1])
    out = sys.argv[2]
    os.makedirs(out, exist_ok=True)
    n = int(FPS * SECONDS)
    for i in range(n):
        with open(os.path.join(out, f"f{i:04d}.svg"), "w") as f:
            f.write(frame(keys, i / FPS))
    print(f"{n} frames, {len(keys)} keys")


if __name__ == "__main__":
    main()
