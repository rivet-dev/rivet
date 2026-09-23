from pathlib import Path
import json
import argparse
import tempfile
import xml.etree.ElementTree as ET
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.font_manager import FontProperties
from matplotlib.lines import Line2D
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
import cairosvg

parser = argparse.ArgumentParser(description="Render Pi runner RSS using Rivet styling")
parser.add_argument("results", type=Path)
parser.add_argument("--output", type=Path, required=True)
parser.add_argument(
    "--website",
    type=Path,
    help="Optional Rivet website directory for Manrope and JetBrains Mono fonts",
)
args = parser.parse_args()
site = args.website
out = args.output
out.mkdir(parents=True, exist_ok=True)
font_temp = tempfile.TemporaryDirectory(prefix="pi-chart-fonts-")
bench = Path(font_temp.name)
data = json.loads(args.results.read_text())
rows = data["rows"]
mib = 1024**2
count = data["verifiedResponses"]
assert (
    count >= 2
    and len(rows) == count + 1
    and data["llmRequests"] == count
    and data["uniquePiSessions"] == count
)
assert [r["live"] for r in rows] == list(range(count + 1))
x = [r["live"] for r in rows]
y = [r["rss"] / mib for r in rows]
base, last = y[0], y[-1]
delta = last - base
fonts = {}
for name, weight, src in [
    ("body", 500, "manrope/Manrope-Variable-latin.woff2"),
    ("bold", 700, "manrope/Manrope-Variable-latin.woff2"),
    ("mono", 400, "jetbrains-mono/JetBrainsMono-Variable-latin.woff2"),
]:
    if site is None:
        fonts[name] = FontProperties(
            family="DejaVu Sans Mono" if name == "mono" else "DejaVu Sans",
            weight="bold" if name == "bold" else "normal",
        )
        continue
    dest = bench / f"{name}.ttf"
    font = instantiateVariableFont(
        TTFont(site / "public/fonts" / src), {"wght": weight}, inplace=True
    )
    # Matplotlib keys SVG glyphs by PostScript name. Variable-font instances
    # retain the same name unless renamed, mixing regular and bold glyphs.
    for record in font["name"].names:
        if record.nameID == 6:
            record.string = f"PiChart-{name}-{weight}".encode(record.getEncoding())
    font.flavor = None
    font.save(dest)
    fonts[name] = FontProperties(fname=str(dest))
PAPER = "#EFEFEF"
INK = "#1B1916"
SOFT = "#56524A"
PINE = "#2E4034"
ACCENT = "#CB5A33"
GRID = "#DCDCDE"
plt.rcParams.update(
    {
        "svg.fonttype": "path",
        "axes.edgecolor": GRID,
        "text.color": INK,
        "axes.labelcolor": SOFT,
        "xtick.color": SOFT,
        "ytick.color": SOFT,
    }
)
fig = plt.figure(figsize=(14, 8), facecolor=PAPER)


def label(x, y, text, size=14, color=INK, font="body", **kw):
    return fig.text(
        x, y, text, fontproperties=fonts[font], fontsize=size, color=color, **kw
    )


def rule(y):
    fig.add_artist(
        Line2D([0.055, 0.945], [y, y], transform=fig.transFigure, color=GRID, lw=1)
    )


label(0.055, 0.89, f"{count} Pi sessions using Rivet Actors", 27, font="bold")
for xpos, value, title, color in [
    (0.055, f"+{delta:.1f} MiB", "RSS increase", ACCENT),
    (0.50, f"{delta / count:.2f} MiB", "Average per session", INK),
]:
    label(xpos, 0.79, title, 13, SOFT)
    label(xpos, 0.725, value, 31, color, "bold")
rule(0.675)
ax = fig.add_axes([0.095, 0.16, 0.81, 0.45], facecolor=PAPER)
ax.fill_between(x, base, y, color=PINE, alpha=0.055, linewidth=0)
ax.plot(x, y, color=PINE, lw=2.5, zorder=3)
ax.scatter(x, y, s=10, color=PINE, zorder=4, edgecolors="none")
ax.scatter(
    [count], [last], s=65, color=ACCENT, zorder=5, edgecolors=PAPER, linewidth=1.5
)
ax.axhline(base, color=SOFT, lw=1, ls=(0, (4, 5)))
span = max(y) - min(y)
ax.set(xlim=(0, count * 1.08), ylim=(min(y) - span * 0.16, max(y) + span * 0.24))
from matplotlib.ticker import MaxNLocator

ax.xaxis.set_major_locator(MaxNLocator(nbins=5, integer=True))
ax.yaxis.set_major_locator(MaxNLocator(nbins=6))
ax.grid(axis="y", color=GRID, lw=0.8)
ax.set_axisbelow(True)
for spine in ["top", "right", "left"]:
    ax.spines[spine].set_visible(False)
ax.spines["bottom"].set_color(GRID)
ax.tick_params(axis="both", length=0, pad=10, labelsize=12)
for tick in [*ax.get_xticklabels(), *ax.get_yticklabels()]:
    tick.set_fontproperties(fonts["mono"])
    tick.set_fontsize(11)
ax.text(
    0,
    1.045,
    "RUNNER RSS · MiB",
    transform=ax.transAxes,
    fontproperties=fonts["mono"],
    fontsize=11,
    color=SOFT,
)
ax.set_xlabel(
    "Live Pi sessions", fontproperties=fonts["body"], fontsize=14, labelpad=15
)
ax.annotate(
    f"{last:.1f} MiB",
    xy=(count, last),
    xytext=(0, 15),
    textcoords="offset points",
    ha="center",
    fontproperties=fonts["bold"],
    fontsize=15,
    color=ACCENT,
)
ax.text(
    count * 0.44,
    base - span * 0.075,
    f"{base:.1f} MiB empty runner",
    fontproperties=fonts["mono"],
    fontsize=11,
    color=SOFT,
    va="top",
    bbox={"facecolor": PAPER, "edgecolor": "none", "pad": 3},
)
svg = out / "pi-session-runner-rss.svg"
fig.savefig(svg, facecolor=PAPER)
# Embed the website's actual vector wordmark; retain vector paths in the export.
ns = "http://www.w3.org/2000/svg"
ET.register_namespace("", ns)
root = ET.fromstring(svg.read_text())
logo = ET.parse(Path(__file__).parent / "assets/rivet.svg").getroot()
logo.set("x", "840")
logo.set("y", "33")
logo.set("width", "112")
logo.set("height", str(112 / 3))
root.append(logo)
ET.ElementTree(root).write(svg, encoding="unicode", xml_declaration=True)
cairosvg.svg2png(
    url=str(svg),
    write_to=str(out / "pi-session-runner-rss.png"),
    output_width=2520,
    output_height=1440,
)
cairosvg.svg2pdf(url=str(svg), write_to=str(out / "pi-session-runner-rss.pdf"))
print(
    f"Rendered PNG, SVG, PDF. {base:.1f} → {last:.1f} MiB; +{delta:.1f} MiB. All {count + 1} samples retained."
)
