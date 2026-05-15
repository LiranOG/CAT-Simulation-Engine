"""
logic_plotter.py - Standalone Analytical Plotting for CAT Simulation Data
==========================================================================
Generates publication-quality static figures from simulation output.
For those who prefer matplotlib's precision over Streamlit's interactivity,
or who need camera-ready figures for papers documenting cosmic loneliness.

Usage:
    python logic_plotter.py --data-dir ../data --output-dir ../data/figures
"""

import argparse
import json
from pathlib import Path

import matplotlib.pyplot as plt
import matplotlib.ticker as mticker
import numpy as np
import pandas as pd
import seaborn as sns

# Plot style configuration.
plt.style.use("dark_background")
sns.set_context("paper", font_scale=1.2)

COLORS = {
    "energy": "#00d2ff",
    "tribalism": "#ff6b6b",
    "collectivism": "#7b2ff7",
    "transcended": "#6bcb77",
    "collapsed": "#ff6b6b",
    "active": "#00d2ff",
    "gap": "#ffd93d",
    "resource_depletion": "#ff8fab",
    "exogenous": "#4cc9f0",
    "background": "#0E1117",
    "grid": "#30363d",
}


def load_data(data_dir: str) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame, dict]:
    """Load all simulation output files from the data directory."""
    data_dir = Path(data_dir)

    tick_df = pd.DataFrame()
    csv_path = data_dir / "tick_history.csv"
    if csv_path.exists():
        tick_df = pd.read_csv(csv_path)
    elif (data_dir / "tick_history.json").exists():
        with open(data_dir / "tick_history.json") as f:
            tick_df = pd.DataFrame(json.load(f))

    collapse_df = pd.DataFrame()
    csv_path = data_dir / "collapse_log.csv"
    if csv_path.exists():
        collapse_df = pd.read_csv(csv_path)
    elif (data_dir / "collapse_log.json").exists():
        with open(data_dir / "collapse_log.json") as f:
            collapse_df = pd.DataFrame(json.load(f))

    agents_df = pd.DataFrame()
    csv_path = data_dir / "final_agents.csv"
    if csv_path.exists():
        agents_df = pd.read_csv(csv_path)

    config = {}
    config_path = data_dir / "simulation_config.json"
    if config_path.exists():
        with open(config_path) as f:
            config = json.load(f)

    return tick_df, collapse_df, agents_df, config


def plot_asynchronous_gap_theory(output_dir: Path) -> None:
    """
    Plot the theoretical Asynchronous Gap: E = e^x vs Maturity = ln(x).

    This is the central figure of CAT: the divergence between exponential
    technological capacity and logarithmic psychological evolution.
    The shaded region is where civilizations die.
    """
    fig, ax = plt.subplots(figsize=(12, 7), facecolor=COLORS["background"])
    ax.set_facecolor(COLORS["background"])

    x = np.linspace(0.1, 25, 1000)
    e_curve = np.exp(x * 0.12)
    maturity_curve = np.log(x + 1) * 3.0

    ax.plot(x, e_curve, color=COLORS["energy"], linewidth=2.5,
            label=r"Technology: $E(t) = e^{0.12t}$")
    ax.plot(x, maturity_curve, color=COLORS["collectivism"], linewidth=2.5,
            label=r"Maturity: $M(t) = 3 \cdot \ln(t+1)$")

    # Shade the Asynchronous Gap region
    crossover_idx = np.argmax(e_curve > maturity_curve)
    if crossover_idx > 0:
        ax.fill_between(
            x[crossover_idx:], maturity_curve[crossover_idx:],
            e_curve[crossover_idx:],
            alpha=0.15, color=COLORS["collapsed"],
            label="Asynchronous Gap (Collapse Zone)",
        )
        ax.axvline(x=x[crossover_idx], color=COLORS["gap"],
                   linestyle="--", alpha=0.7, label="Critical Crossover Point")

    ax.set_yscale("log")
    ax.set_xlabel("Time (arbitrary units)", fontsize=13)
    ax.set_ylabel("Magnitude (log scale)", fontsize=13)
    ax.set_title("The Asynchronous Gap: Exponential Technology vs. "
                 "Logarithmic Maturity", fontsize=15, fontweight="bold")
    ax.legend(loc="upper left", fontsize=10, framealpha=0.3)
    ax.grid(True, alpha=0.2, color=COLORS["grid"])

    fig.tight_layout()
    fig.savefig(output_dir / "asynchronous_gap_theory.png", dpi=300,
                bbox_inches="tight", facecolor=COLORS["background"])
    plt.close(fig)
    print("  wrote asynchronous_gap_theory.png")


def plot_population_dynamics(tick_df: pd.DataFrame, output_dir: Path) -> None:
    """Plot active, collapsed, and transcended population curves over time."""
    if tick_df.empty:
        return

    fig, ax = plt.subplots(figsize=(14, 6), facecolor=COLORS["background"])
    ax.set_facecolor(COLORS["background"])

    ax.fill_between(tick_df["tick"], tick_df["active_agents"],
                    alpha=0.3, color=COLORS["active"])
    ax.plot(tick_df["tick"], tick_df["active_agents"],
            color=COLORS["active"], linewidth=1.5, label="Active")

    if "collapsed_count" in tick_df.columns:
        ax.plot(tick_df["tick"], tick_df["collapsed_count"],
                color=COLORS["collapsed"], linewidth=1.5,
                linestyle="--", label="Collapsed (cumulative)")

    if "transcended_count" in tick_df.columns:
        ax.plot(tick_df["tick"], tick_df["transcended_count"],
                color=COLORS["transcended"], linewidth=1.5,
                linestyle="-.", label="Transcended (cumulative)")

    ax.set_xlabel("Simulation Tick", fontsize=12)
    ax.set_ylabel("Agent Count", fontsize=12)
    ax.set_title("Population Dynamics", fontsize=14, fontweight="bold")
    ax.legend(fontsize=10, framealpha=0.3)
    ax.grid(True, alpha=0.2, color=COLORS["grid"])

    fig.tight_layout()
    fig.savefig(output_dir / "population_dynamics.png", dpi=300,
                bbox_inches="tight", facecolor=COLORS["background"])
    plt.close(fig)
    print("  wrote population_dynamics.png")


def plot_state_vector_evolution(tick_df: pd.DataFrame, output_dir: Path) -> None:
    """Plot mean E, T, C evolution as a 3-panel figure."""
    if tick_df.empty:
        return

    fig, axes = plt.subplots(3, 1, figsize=(14, 12), sharex=True,
                              facecolor=COLORS["background"])

    metrics = [
        ("mean_energy", "Mean Energy (E)", COLORS["energy"]),
        ("mean_tribalism", "Mean Tribalism (T)", COLORS["tribalism"]),
        ("mean_collectivism", "Mean Collectivism (C)", COLORS["collectivism"]),
    ]

    for ax, (col, label, color) in zip(axes, metrics):
        ax.set_facecolor(COLORS["background"])
        if col in tick_df.columns:
            ax.plot(tick_df["tick"], tick_df[col], color=color, linewidth=1.5)
            ax.fill_between(tick_df["tick"], tick_df[col], alpha=0.2, color=color)
        ax.set_ylabel(label, fontsize=11)
        ax.grid(True, alpha=0.2, color=COLORS["grid"])

    axes[-1].set_xlabel("Simulation Tick", fontsize=12)
    axes[0].set_title("State Vector Evolution Over Time",
                       fontsize=14, fontweight="bold")

    fig.tight_layout()
    fig.savefig(output_dir / "state_vector_evolution.png", dpi=300,
                bbox_inches="tight", facecolor=COLORS["background"])
    plt.close(fig)
    print("  wrote state_vector_evolution.png")


def plot_collapse_analysis(collapse_df: pd.DataFrame, output_dir: Path) -> None:
    """Generate collapse event analysis plots."""
    if collapse_df.empty:
        return

    # E-T scatter at collapse
    fig, ax = plt.subplots(figsize=(10, 8), facecolor=COLORS["background"])
    ax.set_facecolor(COLORS["background"])

    type_colors = {
        "AsynchronousGap": COLORS["collapsed"],
        "ResourceDepletion": COLORS["resource_depletion"],
        "ExogenousExtinction": COLORS["exogenous"],
    }

    for ctype, color in type_colors.items():
        mask = collapse_df["collapse_type"] == ctype
        if mask.any():
            subset = collapse_df[mask]
            ax.scatter(subset["energy"], subset["tribalism"],
                       c=color, s=subset["collectivism"] * 100 + 10,
                       alpha=0.5, label=ctype, edgecolors="white",
                       linewidths=0.3)

    ax.set_xlabel("Energy at Collapse", fontsize=12)
    ax.set_ylabel("Tribalism at Collapse", fontsize=12)
    ax.set_title("State Vectors at Moment of Civilizational Collapse",
                 fontsize=14, fontweight="bold")
    ax.legend(fontsize=10, framealpha=0.3)
    ax.grid(True, alpha=0.2, color=COLORS["grid"])

    fig.tight_layout()
    fig.savefig(output_dir / "collapse_scatter.png", dpi=300,
                bbox_inches="tight", facecolor=COLORS["background"])
    plt.close(fig)
    print("  wrote collapse_scatter.png")

    # Collapse timing histogram
    fig, ax = plt.subplots(figsize=(12, 5), facecolor=COLORS["background"])
    ax.set_facecolor(COLORS["background"])
    ax.hist(collapse_df["tick"], bins=50, color=COLORS["collapsed"],
            alpha=0.7, edgecolor="white", linewidth=0.5)
    ax.set_xlabel("Simulation Tick", fontsize=12)
    ax.set_ylabel("Collapse Events", fontsize=12)
    ax.set_title("Temporal Distribution of Collapse Events",
                 fontsize=14, fontweight="bold")
    ax.grid(True, alpha=0.2, color=COLORS["grid"])

    fig.tight_layout()
    fig.savefig(output_dir / "collapse_histogram.png", dpi=300,
                bbox_inches="tight", facecolor=COLORS["background"])
    plt.close(fig)
    print("  wrote collapse_histogram.png")


def plot_spatial_distribution(
    agents_df: pd.DataFrame, collapse_df: pd.DataFrame, output_dir: Path
) -> None:
    """Plot spatial distribution of final agent states and collapse sites."""
    if agents_df.empty:
        return

    fig, ax = plt.subplots(figsize=(10, 10), facecolor=COLORS["background"])
    ax.set_facecolor(COLORS["background"])

    state_colors = {
        "Nascent": "#8b949e",
        "Evolving": COLORS["active"],
        "Transcended": COLORS["transcended"],
        "Collapsed": COLORS["collapsed"],
    }

    for state, color in state_colors.items():
        mask = agents_df["state"] == state
        if mask.any():
            subset = agents_df[mask]
            ax.scatter(subset["pos_x"], subset["pos_y"],
                       c=color, s=10, alpha=0.5, label=state)

    ax.set_xlabel("X", fontsize=12)
    ax.set_ylabel("Y", fontsize=12)
    ax.set_title("Final Spatial Distribution of Civilizations",
                 fontsize=14, fontweight="bold")
    ax.legend(fontsize=10, framealpha=0.3, loc="upper right")
    ax.grid(True, alpha=0.1, color=COLORS["grid"])
    ax.set_aspect("equal")

    fig.tight_layout()
    fig.savefig(output_dir / "spatial_distribution.png", dpi=300,
                bbox_inches="tight", facecolor=COLORS["background"])
    plt.close(fig)
    print("  wrote spatial_distribution.png")


def main() -> None:
    """Parse CLI arguments, load one run archive, and write static figures."""
    parser = argparse.ArgumentParser(
        description="Generate publication-quality figures from CAT simulation data."
    )
    parser.add_argument("--data-dir", type=str, default="../data",
                        help="Path to simulation output directory.")
    parser.add_argument("--output-dir", type=str, default=None,
                        help="Path for output figures (default: <data-dir>/figures).")
    args = parser.parse_args()

    output_dir = Path(args.output_dir) if args.output_dir else Path(args.data_dir) / "figures"
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Loading data from: {args.data_dir}")
    tick_df, collapse_df, agents_df, config = load_data(args.data_dir)

    print(f"Generating figures -> {output_dir}")
    plot_asynchronous_gap_theory(output_dir)
    plot_population_dynamics(tick_df, output_dir)
    plot_state_vector_evolution(tick_df, output_dir)
    plot_collapse_analysis(collapse_df, output_dir)
    plot_spatial_distribution(agents_df, collapse_df, output_dir)

    print(f"\nAll figures saved to: {output_dir}")


if __name__ == "__main__":
    main()
