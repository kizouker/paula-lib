"""Small examples in a neon "glow" theme: aliasing, quantization with and
without dither, and a Monte Carlo check of E[nu|z]=0 and E[nu^2|z]=q^2/4.
Run: uv run --with numpy --with matplotlib demo.py
"""
import os
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "figures")
os.makedirs(OUT, exist_ok=True)
rng = np.random.default_rng(1)

BG, GRID, TEXT = "#0b0f1a", "#1c2438", "#d6e2ff"
CYAN, PINK, GREEN, ORANGE, WHITE = "#00e5ff", "#ff2e97", "#39ff88", "#ffb02e", "#ffffff"

plt.rcParams.update({
    "figure.facecolor": BG, "axes.facecolor": BG, "savefig.facecolor": BG,
    "axes.edgecolor": GRID, "axes.labelcolor": TEXT, "xtick.color": TEXT,
    "ytick.color": TEXT, "text.color": TEXT, "axes.titlecolor": TEXT,
    "grid.color": GRID, "axes.grid": True, "legend.facecolor": BG,
    "legend.edgecolor": GRID,
})


def glow(ax, x, y, color, lw=1.6, step=False, **kw):
    """Draw a line with a soft halo: wide faint strokes under a sharp core."""
    draw = ax.step if step else ax.plot
    if step:
        kw.setdefault("where", "mid")
    for w, a in ((9, 0.05), (6, 0.08), (3.5, 0.14)):
        draw(x, y, color=color, lw=lw + w, alpha=a, solid_capstyle="round", **kw)
    draw(x, y, color=color, lw=lw, **kw)


def glow_pts(ax, x, y, color, label=None):
    for s, a in ((260, 0.06), (140, 0.12), (60, 0.25)):
        ax.scatter(x, y, s=s, color=color, alpha=a, linewidths=0, zorder=3)
    ax.scatter(x, y, s=16, color=WHITE, zorder=4, label=label)


def quant(u, q):
    return q * np.floor(u / q + 0.5)


# --- Fig 1: aliasing, 7000 Hz sampled at 8363 Hz -> looks like 1363 Hz -------
fs, f = 8363.0, 7000.0
fa = f - fs * round(f / fs)
t_dense = np.linspace(0, 1.5e-3, 3000)
n = np.arange(0, int(1.5e-3 * fs) + 1)
t_s = n / fs
fig, ax = plt.subplots(figsize=(9, 3.4))
glow(ax, t_dense * 1e3, np.sin(2 * np.pi * f * t_dense), CYAN, 1.0, label="7000 Hz tone")
glow(ax, t_dense * 1e3, np.sin(2 * np.pi * fa * t_dense), PINK, 1.8, label=f"alias {abs(fa):.0f} Hz")
glow_pts(ax, t_s * 1e3, np.sin(2 * np.pi * f * t_s), WHITE, "samples at 8363 Hz")
ax.set_xlabel("time (ms)"); ax.set_title("Aliasing: the dots fit both curves")
ax.legend(loc="upper right")
fig.tight_layout(); fig.savefig(os.path.join(OUT, "1_alias.png"), dpi=130); plt.close(fig)

# --- Fig 2: 3-bit quantization of a decaying sine, plain vs TPDF -------------
bits = 3
q = 2.0 / 2 ** bits
N = 600
k = np.arange(N)
x = 0.9 * np.exp(-3.2 * k / N) * np.sin(2 * np.pi * 6 * k / N)
y0 = quant(x, q)
d = rng.uniform(-q / 2, q / 2, N) + rng.uniform(-q / 2, q / 2, N)
y1 = quant(x + d, q)
fig, axs = plt.subplots(3, 1, figsize=(9, 7.4), sharex=True)
glow(axs[0], k, x, CYAN, 1.0, label="input")
glow(axs[0], k, y0, PINK, 1.4, step=True, label="no dither")
axs[0].set_title(f"{bits}-bit quantizer, q={q}"); axs[0].legend(loc="upper right")
glow(axs[1], k, x, CYAN, 1.0, label="input")
glow(axs[1], k, y1, GREEN, 1.0, step=True, label="TPDF dither")
axs[1].legend(loc="upper right")
glow(axs[2], k, y0 - x, PINK, 1.0, label="error, no dither (follows signal)")
glow(axs[2], k, y1 - x, GREEN, 0.7, label="error, TPDF (noise-like)")
axs[2].legend(loc="upper right"); axs[2].set_xlabel("sample")
fig.tight_layout(); fig.savefig(os.path.join(OUT, "2_quantize.png"), dpi=130); plt.close(fig)

# --- Fig 3: Monte Carlo of E[nu|z] and E[nu^2|z] over z in [0, q) ------------
q = 1.0
zs = np.linspace(0, q, 41)
M = 200_000
rows = {"none": [], "RPDF": [], "TPDF": []}
for z in zs:
    for name in rows:
        if name == "none":
            d = np.zeros(M)
        elif name == "RPDF":
            d = rng.uniform(-q / 2, q / 2, M)
        else:
            d = rng.uniform(-q / 2, q / 2, M) + rng.uniform(-q / 2, q / 2, M)
        nu = quant(z + d, q) - z
        rows[name].append((nu.mean(), (nu ** 2).mean()))
fig, axs = plt.subplots(1, 2, figsize=(10, 3.8))
for name, col in (("none", PINK), ("RPDF", ORANGE), ("TPDF", GREEN)):
    arr = np.array(rows[name])
    glow(axs[0], zs, arr[:, 0], col, 1.4, label=name)
    glow(axs[1], zs, arr[:, 1], col, 1.4, label=name)
axs[1].axhline(q ** 2 / 4, color=WHITE, ls="--", lw=0.8, alpha=0.8, label="q^2/4")
axs[0].set_title("E[nu | z]"); axs[1].set_title("E[nu^2 | z]")
for a in axs:
    a.set_xlabel("z (fraction of q)"); a.legend()
fig.tight_layout(); fig.savefig(os.path.join(OUT, "3_moments.png"), dpi=130); plt.close(fig)

tp = np.array(rows["TPDF"])
print("TPDF  mean(nu) range: %.4f .. %.4f" % (tp[:, 0].min(), tp[:, 0].max()))
print("TPDF  E[nu^2] range : %.4f .. %.4f (theory 0.25)" % (tp[:, 1].min(), tp[:, 1].max()))
rp = np.array(rows["RPDF"])
print("RPDF  E[nu^2] range : %.4f .. %.4f (depends on z)" % (rp[:, 1].min(), rp[:, 1].max()))
