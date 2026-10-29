"""Build the Lit and Off mouse illustrations from the user's ink SVGs.

cls-1 glow, cls-2 outline, cls-3 LEDs and logo; adds a G cut out of each logo.
Writes top-lit.svg, top-off.svg, side-lit.svg, side-off.svg (no card behind).
"""
import re

CARD = "#1D1D20"
G_TOP = (f'<path d="M172.2 507.6 A15 15 0 1 0 176.6 518.2 L163.2 518.2" fill="none" stroke="{CARD}" '
         f'stroke-width="6.5" stroke-linecap="round" stroke-linejoin="round"/>')
G_SIDE = (f'<g transform="translate(546 24) rotate(14) scale(1.2 0.46)"><path d="M10.6 -10.6 A15 15 0 1 0 15 0 L1.6 0" '
          f'fill="none" stroke="{CARD}" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"/></g>')
STYLES = {
    "lit": (".cls-1{fill:#26A269;fill-opacity:0.22}.cls-2{fill:#26A269}.cls-3{fill:#26A269}"),
    "off": (".cls-1{fill:#FFFFFF;fill-opacity:0}.cls-2{fill:#FFFFFF;fill-opacity:0.35}.cls-3{fill:#FFFFFF;fill-opacity:0.2}"),
}
for view, g in (("top", G_TOP), ("side", G_SIDE)):
    src = open(f"g502-{view}-ink.svg").read()
    for mode, style in STYLES.items():
        s = re.sub(r"<style>[\s\S]*?</style>", f"<style>{style}</style>", src)
        s = re.sub(r"<title>[\s\S]*?</title>", "", s)
        s = s.replace("</svg>", g + "</svg>")
        open(f"{view}-{mode}.svg", "w").write(s)
print("built")
