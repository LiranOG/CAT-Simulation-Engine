"""
dashboard.py - CAT Simulation Dashboard (Dark Mode, Multi-Run)
==============================================================
ARCHITECTURE NOTES:
  - A single resolved_base (Path) is the authoritative base dir throughout.
  - run data lives EXCLUSIVELY in resolved_base/runs/<timestamp_seed_n>/.
  - Orphaned files directly in resolved_base/ are pre-archive historical
    artifacts; they are reported but never loaded.

BUG FIXES in this version:
  1. DARK_LAYOUT excludes 'xaxis'/'yaxis'; axes use update_xaxes/update_yaxes
     only (eliminates 'multiple values for keyword argument' TypeError).
  2. selectbox bound to key="run_selector" for reactive cache invalidation.
  3. Comparison table used undefined 'base_data_dir'; replaced with
     resolved_base everywhere (NameError at line 293 is now impossible).
"""

import json
from pathlib import Path

import numpy as np
import pandas as pd
import plotly.express as px
import plotly.graph_objects as go
from plotly.subplots import make_subplots
import streamlit as st

# Page configuration.
st.set_page_config(
    page_title="CAT Simulation Dashboard",
    page_icon="CAT",
    layout="wide",
    initial_sidebar_state="expanded",
)

st.markdown(
    """
<style>
    .stApp { background-color: #0E1117; color: #FAFAFA; }
    .stSidebar { background-color: #161B22; }
    .metric-card {
        background: linear-gradient(135deg,#1a1a2e 0%,#16213e 100%);
        border: 1px solid #30363d; border-radius: 12px;
        padding: 20px; margin: 8px 0; text-align: center;
    }
    .metric-value {
        font-size: 2.4rem; font-weight: 700;
        background: linear-gradient(90deg,#00d2ff,#7b2ff7);
        -webkit-background-clip: text; -webkit-text-fill-color: transparent;
    }
    .metric-label {
        font-size: 0.85rem; color: #8b949e;
        text-transform: uppercase; letter-spacing: 1.5px; margin-top: 4px;
    }
    .header-gradient {
        background: linear-gradient(90deg,#00d2ff 0%,#7b2ff7 50%,#ff6b6b 100%);
        -webkit-background-clip: text; -webkit-text-fill-color: transparent;
        font-size: 2.8rem; font-weight: 800;
        text-align: center; margin-bottom: 0.5rem;
    }
    .subheader { text-align:center; color:#8b949e; font-size:1.1rem; margin-bottom:2rem; }
    .run-badge {
        background:#161B22; border:1px solid #30363d; border-radius:8px;
        padding:8px 14px; font-size:0.8rem; color:#8b949e; margin-bottom:1rem;
    }
</style>
""",
    unsafe_allow_html=True,
)

# Theme constants.
# DARK_LAYOUT intentionally excludes 'xaxis' and 'yaxis'.
# Axis styling is applied only through update_xaxes() / update_yaxes()
# to avoid 'multiple values for keyword argument' collisions.
DARK_LAYOUT = dict(
    paper_bgcolor="#0E1117",
    plot_bgcolor="#161B22",
    font=dict(color="#FAFAFA", family="Inter, sans-serif"),
    legend=dict(bgcolor="rgba(22,33,62,0.8)", bordercolor="#30363d", borderwidth=1),
    colorway=[
        "#00d2ff",
        "#7b2ff7",
        "#ff6b6b",
        "#ffd93d",
        "#6bcb77",
        "#ff8fab",
        "#4cc9f0",
        "#f72585",
    ],
)

# Axis styling is applied via update_xaxes / update_yaxes, never via layout kwargs.
DARK_AXES = dict(gridcolor="#30363d", zerolinecolor="#30363d", linecolor="#30363d")

COLORS = dict(
    energy="#00d2ff",
    tribalism="#ff6b6b",
    collectivism="#7b2ff7",
    transcended="#6bcb77",
    collapsed="#ff6b6b",
    active="#00d2ff",
    gap="#ffd93d",
)


def dark_fig(fig: go.Figure, height: int = 500, **layout_extra) -> go.Figure:
    """
    Apply dark theme to a figure.
    - layout_extra may contain title, xaxis_title, yaxis_title, etc.
    - Never pass 'xaxis' or 'yaxis' dicts here; use update_xaxes/update_yaxes instead.
    """
    fig.update_layout(**DARK_LAYOUT, height=height, **layout_extra)
    fig.update_xaxes(**DARK_AXES)
    fig.update_yaxes(**DARK_AXES)
    return fig


def metric_html(label: str, value: str) -> str:
    """Render one numeric metric as an HTML card for Streamlit markdown."""
    return (
        f'<div class="metric-card">'
        f'<div class="metric-value">{value}</div>'
        f'<div class="metric-label">{label}</div>'
        f"</div>"
    )


# Run discovery is not cached because filesystem state changes between reruns.
def discover_runs(resolved_base: Path) -> tuple[list, list]:
    """
    Scan resolved_base/runs/ and return (completed_run_names, incomplete_run_names).

    A run is COMPLETED if it contains RUN_MANIFEST.json, written as the last
    action of a successful simulation. Absence of the manifest means the process
    was interrupted (crash, OOM, SIGTERM) before all final files were flushed.

    Returns
    -------
    completed   : list[str]   names of runs with RUN_MANIFEST.json, newest-first
    incomplete  : list[str]   names of run directories missing the manifest
    """
    runs_root = resolved_base / "runs"
    if not runs_root.is_dir():
        st.session_state["incomplete_runs"] = []
        return []
    completed, incomplete = [], []
    for d in runs_root.iterdir():
        if not d.is_dir():
            continue
        if (d / "RUN_MANIFEST.json").exists():
            completed.append(d.name)
        else:
            incomplete.append(d.name)
    st.session_state["incomplete_runs"] = sorted(incomplete, reverse=True)
    return sorted(completed, reverse=True)


def has_orphaned_data(resolved_base: Path) -> bool:
    """True if CSV or JSON files exist directly in the base dir (pre-archive runs)."""
    for ext in ("*.csv", "*.json"):
        if any(resolved_base.glob(ext)):
            return True
    return False


def orphan_cleanup_suggestion(resolved_base: Path) -> str:
    """
    Returns a human-readable message explaining what orphaned files are,
    why they exist, and what to do. Called only when has_orphaned_data() is True.

    Orphaned files are data files written directly into the base data directory
    by a version of the engine that predated the archive system. Every run of
    the current engine writes exclusively to a timestamped subdirectory under
    runs/. These files will never be updated again.
    """
    orphans = sorted(
        f.name for ext in ("*.csv", "*.json") for f in resolved_base.glob(ext)
    )
    file_list = ", ".join(orphans[:6])
    if len(orphans) > 6:
        file_list += f" ... (+{len(orphans) - 6} more)"
    return (
        f"**Orphaned pre-archive data detected** in `{resolved_base}`\n\n"
        f"Files: `{file_list}`\n\n"
        "These were written by an older version of the engine that dumped output "
        "directly into the base data directory instead of a timestamped `runs/` subfolder. "
        "The current engine **never** writes here. Every run now goes into "
        "`runs/YYYY-MM-DD_HHMMSS_seed{N}_n{M}/`.\n\n"
        "**To silence this warning**, move or delete these files:\n"
        "```powershell\n"
        f'Remove-Item "{resolved_base}\\*.csv", "{resolved_base}\\*.json" -Force\n'
        "```\n"
        "Run the engine again to generate a properly-archived dataset."
    )


# Data loaders are cached per absolute run directory string.
# Cache key = run_dir string. Different run means different string and cache miss.
# fresh CSV read. This is how Streamlit @st.cache_data achieves reactivity.


@st.cache_data
def load_tick_history(run_dir: str) -> pd.DataFrame:
    """
    Load per-tick aggregate statistics from the completed run directory.
    Tries CSV first (smaller, faster), falls back to JSON.
    Returns an empty DataFrame and surfaces a warning if the file is corrupt.
    """
    for fname, as_json in [("tick_history.csv", False), ("tick_history.json", True)]:
        p = Path(run_dir) / fname
        if p.exists():
            try:
                return (
                    pd.read_csv(p)
                    if not as_json
                    else pd.DataFrame(json.loads(p.read_text()))
                )
            except Exception as exc:
                st.warning(f"⚠️ Could not parse `{fname}`: {exc}")
    return pd.DataFrame()


@st.cache_data
def load_collapse_log(run_dir: str) -> pd.DataFrame:
    """
    Load collapse event records from the completed run directory.
    Returns an empty DataFrame and surfaces a warning if the file is corrupt.
    """
    for fname, as_json in [("collapse_log.csv", False), ("collapse_log.json", True)]:
        p = Path(run_dir) / fname
        if p.exists():
            try:
                return (
                    pd.read_csv(p)
                    if not as_json
                    else pd.DataFrame(json.loads(p.read_text()))
                )
            except Exception as exc:
                st.warning(f"⚠️ Could not parse `{fname}`: {exc}")
    return pd.DataFrame()


@st.cache_data
def load_final_agents(run_dir: str) -> pd.DataFrame:
    """
    Load terminal agent state records from the completed run directory.
    Returns an empty DataFrame and surfaces a warning if the file is corrupt.
    """
    p = Path(run_dir) / "final_agents.csv"
    if not p.exists():
        return pd.DataFrame()
    try:
        return pd.read_csv(p)
    except Exception as exc:
        st.warning(f"⚠️ Could not parse `final_agents.csv`: {exc}")
        return pd.DataFrame()


@st.cache_data
def load_config(run_dir: str) -> dict:
    p = Path(run_dir) / "simulation_config.json"
    if not p.exists():
        return {}
    try:
        return json.loads(p.read_text())
    except Exception as exc:
        st.warning(f"⚠️ Could not parse `simulation_config.json`: {exc}")
        return {}


@st.cache_data
def load_manifest(run_dir: str) -> dict:
    """Load the RUN_MANIFEST.json completion record for a run."""
    p = Path(run_dir) / "RUN_MANIFEST.json"
    return json.loads(p.read_text()) if p.exists() else {}


# Sidebar.
st.sidebar.markdown("## CAT Simulation Dashboard")
st.sidebar.markdown("---")

base_data_dir_raw = st.sidebar.text_input(
    "Base Data Directory",
    value="../data",
    help="Relative or absolute path to the directory containing the runs/ folder.",
)

# Resolve to an absolute path to eliminate relative-path ambiguity.
resolved_base: Path = Path(base_data_dir_raw).resolve()
st.sidebar.caption(f"`{resolved_base}`")

available_runs = discover_runs(resolved_base)
incomplete_runs = st.session_state.get("incomplete_runs", [])

if has_orphaned_data(resolved_base):
    with st.sidebar.expander("Orphaned files detected", expanded=False):
        st.warning(
            f"CSV/JSON files exist directly in `{resolved_base}`.\n"
            "These are from a pre-archive engine run and are NOT loaded by the dashboard."
        )

if incomplete_runs:
    with st.sidebar.expander(
        f"⚠️ {len(incomplete_runs)} incomplete run(s)", expanded=False
    ):
        st.warning(
            "The following run directories are **missing `RUN_MANIFEST.json`**, "
            "indicating the simulation process was interrupted before completing "
            "all final exports (crash, OOM, or SIGTERM).\n\n"
            "These runs are **not loaded** because their data may be partial or corrupt.\n\n"
            + "\n".join(f"- `{r}`" for r in incomplete_runs)
        )

# Empty state.
if not available_runs:
    st.markdown(
        '<div class="header-gradient">CAT Simulation Dashboard</div>',
        unsafe_allow_html=True,
    )
    st.markdown(
        '<div class="subheader">Cosmobiological Asynchrony Theory - '
        "Fermi Paradox Agent-Based Model</div>",
        unsafe_allow_html=True,
    )
    st.info(
        "**No simulation runs detected.**\n\n"
        f"Scanned: `{resolved_base / 'runs'}`\n\n"
        "Run the engine to generate data:\n"
        "```\ncd engine_rust\ncargo run --release -- -t 10000 -n 1000\n```",
        icon=":material/search:",
    )
    st.stop()

# Run selector.
# key="run_selector" binds the widget to st.session_state["run_selector"].
# On every script rerun, `selected_run` is the widget's current value.
selected_run: str = st.sidebar.selectbox(
    "Select Run",
    options=available_runs,
    index=0,
    key="run_selector",
    help="Runs sorted newest-first. Switching instantly reloads all data.",
)

st.sidebar.markdown(
    f'<div class="run-badge">{selected_run}</div>', unsafe_allow_html=True
)

page = st.sidebar.radio(
    "View",
    [
        "Overview",
        "State Vectors",
        "Collapse Analysis",
        "Spatial Distribution",
        "Asynchronous Gap",
        "Configuration",
    ],
)

# Load data for the selected run.
# selected_run_dir changes every time selected_run changes.
# @st.cache_data keys on this string; a different string causes a fresh file read.
selected_run_dir: str = str(resolved_base / "runs" / selected_run)

tick_df = load_tick_history(selected_run_dir)
collapse_df = load_collapse_log(selected_run_dir)
agents_df = load_final_agents(selected_run_dir)
config = load_config(selected_run_dir)
manifest = load_manifest(selected_run_dir)
has_data = not tick_df.empty


# -----------------------------------------------------------------------------
# PAGE: Overview
# -----------------------------------------------------------------------------
if page == "Overview":
    st.markdown(
        '<div class="header-gradient">CAT Simulation Dashboard</div>',
        unsafe_allow_html=True,
    )
    st.markdown(
        '<div class="subheader">Cosmobiological Asynchrony Theory - '
        "Fermi Paradox Agent-Based Model</div>",
        unsafe_allow_html=True,
    )
    completed_at = manifest.get("completed_at", "unknown")
    st.markdown(
        f'<div class="run-badge"><strong>{selected_run}</strong>'
        f" &nbsp;|&nbsp; {len(available_runs)} completed run(s)"
        f" &nbsp;|&nbsp; completed {completed_at}</div>",
        unsafe_allow_html=True,
    )

    if not has_data:
        st.warning("Data files not yet available for this run.")
        st.stop()

    last = tick_df.iloc[-1]
    total = max(len(agents_df), 1)
    collapse_rate = int(last.get("collapsed_count", 0)) / total * 100

    c1, c2, c3, c4 = st.columns(4)
    with c1:
        st.markdown(
            metric_html("Total Collapses", f"{int(last.get('collapsed_count', 0)):,}"),
            unsafe_allow_html=True,
        )
    with c2:
        st.markdown(
            metric_html("Transcended", f"{int(last.get('transcended_count', 0)):,}"),
            unsafe_allow_html=True,
        )
    with c3:
        st.markdown(
            metric_html("Still Active", f"{int(last.get('active_agents', 0)):,}"),
            unsafe_allow_html=True,
        )
    with c4:
        st.markdown(
            metric_html("Collapse Rate", f"{collapse_rate:.1f}%"),
            unsafe_allow_html=True,
        )

    st.markdown("---")

    fig = go.Figure()
    fig.add_trace(
        go.Scatter(
            x=tick_df["tick"],
            y=tick_df["active_agents"],
            name="Active",
            fill="tozeroy",
            line=dict(color=COLORS["active"], width=2),
        )
    )
    if "collapsed_count" in tick_df.columns:
        fig.add_trace(
            go.Scatter(
                x=tick_df["tick"],
                y=tick_df["collapsed_count"],
                name="Collapsed (cumulative)",
                line=dict(color=COLORS["collapsed"], width=2, dash="dot"),
            )
        )
    if "transcended_count" in tick_df.columns:
        fig.add_trace(
            go.Scatter(
                x=tick_df["tick"],
                y=tick_df["transcended_count"],
                name="Transcended (cumulative)",
                line=dict(color=COLORS["transcended"], width=2, dash="dash"),
            )
        )
    dark_fig(
        fig,
        height=500,
        title="Population Dynamics Over Time",
        xaxis_title="Simulation Tick",
        yaxis_title="Agent Count",
    )
    st.plotly_chart(fig, use_container_width=True)

    if len(available_runs) > 1:
        st.markdown("### All Runs - Quick Comparison")
        rows = []
        for rn in available_runs:
            # resolved_base is the single authoritative Path; never use raw strings here.
            rd = str(resolved_base / "runs" / rn)
            th = load_tick_history(rd)
            if not th.empty:
                lr = th.iloc[-1]
                rows.append(
                    {
                        "Run": rn,
                        "Ticks": int(lr.get("tick", 0)),
                        "Collapses": int(lr.get("collapsed_count", 0)),
                        "Transcended": int(lr.get("transcended_count", 0)),
                        "Active": int(lr.get("active_agents", 0)),
                        "Mean E": round(float(lr.get("mean_energy", 0)), 4),
                        "Mean T": round(float(lr.get("mean_tribalism", 0)), 4),
                    }
                )
        if rows:
            st.dataframe(pd.DataFrame(rows), use_container_width=True, hide_index=True)


# -----------------------------------------------------------------------------
# PAGE: State Vectors
# -----------------------------------------------------------------------------
elif page == "State Vectors":
    st.markdown("## State Vector Evolution")
    if not has_data:
        st.warning("No data for this run.")
        st.stop()

    fig = make_subplots(
        rows=3,
        cols=1,
        shared_xaxes=True,
        vertical_spacing=0.09,
        subplot_titles=(
            "Mean Energy  E(t) = E0 * exp(r * t)",
            "Mean Tribalism  T(t) = T0 * max(0, 1 - alpha * ln(1 + t))",
            "Mean Collectivism  C(t) = clamp(C0 + delta * t, 0, 1)",
        ),
    )
    fig.add_trace(
        go.Scatter(
            x=tick_df["tick"],
            y=tick_df["mean_energy"],
            name="Energy",
            line=dict(color=COLORS["energy"], width=2),
        ),
        row=1,
        col=1,
    )
    fig.add_trace(
        go.Scatter(
            x=tick_df["tick"],
            y=tick_df["mean_tribalism"],
            name="Tribalism",
            line=dict(color=COLORS["tribalism"], width=2),
        ),
        row=2,
        col=1,
    )
    fig.add_trace(
        go.Scatter(
            x=tick_df["tick"],
            y=tick_df["mean_collectivism"],
            name="Collectivism",
            line=dict(color=COLORS["collectivism"], width=2),
        ),
        row=3,
        col=1,
    )

    fig.update_layout(**DARK_LAYOUT, height=860, showlegend=True)
    fig.update_xaxes(**DARK_AXES)
    fig.update_yaxes(**DARK_AXES)
    st.plotly_chart(fig, use_container_width=True)

    if "max_energy" in tick_df.columns:
        st.markdown("### Peak Energy Envelope")
        fig2 = go.Figure()
        fig2.add_trace(
            go.Scatter(
                x=tick_df["tick"],
                y=tick_df["max_energy"],
                name="Max E",
                line=dict(color=COLORS["gap"], width=2),
            )
        )
        dark_fig(fig2, height=350, xaxis_title="Tick", yaxis_title="Max Energy")
        st.plotly_chart(fig2, use_container_width=True)


# -----------------------------------------------------------------------------
# PAGE: Collapse Analysis
# -----------------------------------------------------------------------------
elif page == "Collapse Analysis":
    st.markdown("## Collapse Event Analysis")
    if collapse_df.empty:
        st.info("No collapse events for this run.")
        st.stop()

    col1, col2 = st.columns(2)
    with col1:
        tc = collapse_df["collapse_type"].value_counts()
        fig = px.pie(
            values=tc.values,
            names=tc.index,
            title="Collapse Type Distribution",
            color_discrete_sequence=["#ff6b6b", "#ffd93d", "#4cc9f0"],
        )
        fig.update_layout(**DARK_LAYOUT)
        st.plotly_chart(fig, use_container_width=True)
    with col2:
        fig = px.histogram(
            collapse_df,
            x="tick",
            nbins=50,
            title="Collapse Events Over Time",
            color_discrete_sequence=["#ff6b6b"],
        )
        dark_fig(fig, height=400, xaxis_title="Tick", yaxis_title="Collapses")
        st.plotly_chart(fig, use_container_width=True)

    st.markdown("### Energy vs Tribalism at Collapse")
    fig = px.scatter(
        collapse_df,
        x="energy",
        y="tribalism",
        color="collapse_type",
        size="collectivism",
        title="State Vectors at Moment of Collapse",
        color_discrete_sequence=["#ff6b6b", "#ffd93d", "#4cc9f0"],
        opacity=0.7,
    )
    dark_fig(fig, height=500)
    st.plotly_chart(fig, use_container_width=True)

    st.markdown("### Statistics at Collapse")
    st.dataframe(
        collapse_df[["energy", "tribalism", "collectivism"]]
        .describe()
        .style.format("{:.4f}"),
        use_container_width=True,
    )


# -----------------------------------------------------------------------------
# PAGE: Spatial Distribution
# -----------------------------------------------------------------------------
elif page == "Spatial Distribution":
    st.markdown("## Spatial Distribution")
    if agents_df.empty:
        st.info("No final agent data for this run.")
        st.stop()

    fig = px.scatter(
        agents_df,
        x="pos_x",
        y="pos_y",
        color="state",
        size="energy",
        title="Final Agent Positions",
        color_discrete_map={
            "Nascent": "#8b949e",
            "Evolving": "#00d2ff",
            "Transcended": "#7b2ff7",
            "Collapsed": "#ff6b6b",
        },
        opacity=0.6,
        size_max=15,
    )
    dark_fig(fig, height=700, xaxis_title="X", yaxis_title="Y")
    st.plotly_chart(fig, use_container_width=True)

    if not collapse_df.empty:
        st.markdown("### Collapse Density Heatmap")
        fig = px.density_heatmap(
            collapse_df,
            x="pos_x",
            y="pos_y",
            title="Where Civilizations Die",
            color_continuous_scale="Inferno",
        )
        dark_fig(fig, height=500)
        st.plotly_chart(fig, use_container_width=True)


# -----------------------------------------------------------------------------
# PAGE: Asynchronous Gap
# -----------------------------------------------------------------------------
elif page == "Asynchronous Gap":
    st.markdown("## The Asynchronous Gap")
    st.markdown(
        "**E = exp(r * t)** outpaces **M = 2.5 * ln(t + 1)**. "
        "The shaded region is where civilizations die."
    )
    if not has_data:
        st.warning("No data for this run.")
        st.stop()

    x = np.linspace(0.1, 30, 600)
    e_curve = np.exp(x * 0.15)
    t_curve = np.log(x + 1) * 2.5

    fig = go.Figure()
    fig.add_trace(
        go.Scatter(
            x=x,
            y=e_curve,
            name="E = exp(0.15 * t)",
            line=dict(color=COLORS["tribalism"], width=3),
        )
    )
    fig.add_trace(
        go.Scatter(
            x=x,
            y=t_curve,
            name="Maturity = 2.5 * ln(t + 1)",
            line=dict(color=COLORS["energy"], width=3),
        )
    )

    crossover = int(np.argmax(e_curve > t_curve))
    if crossover:
        fig.add_vrect(
            x0=float(x[crossover]),
            x1=float(x[-1]),
            fillcolor="#ff6b6b",
            opacity=0.08,
            annotation_text="ASYNCHRONOUS GAP",
            annotation_position="top left",
            annotation_font_color="#ff6b6b",
        )
        fig.add_vline(
            x=float(x[crossover]),
            line_color=COLORS["gap"],
            line_dash="dash",
            opacity=0.7,
        )

    # Apply base dark layout with no yaxis kwarg here.
    fig.update_layout(
        **DARK_LAYOUT,
        height=500,
        title="The Asynchronous Gap",
        xaxis_title="Time (arbitrary units)",
        yaxis_title="Magnitude (log scale)",
    )
    fig.update_xaxes(**DARK_AXES)
    # Log scale applied via update_yaxes to avoid collision with DARK_LAYOUT.
    fig.update_yaxes(**DARK_AXES, type="log")
    st.plotly_chart(fig, use_container_width=True)

    if "mean_async_gap" in tick_df.columns:
        st.markdown("### Simulated Mean Gap  G(t) = E / (1 - T + epsilon)")
        fig2 = go.Figure()
        fig2.add_trace(
            go.Scatter(
                x=tick_df["tick"],
                y=tick_df["mean_async_gap"],
                fill="tozeroy",
                line=dict(color=COLORS["gap"], width=2),
            )
        )
        dark_fig(fig2, height=400, xaxis_title="Tick", yaxis_title="G(t)")
        st.plotly_chart(fig2, use_container_width=True)


# -----------------------------------------------------------------------------
# PAGE: Configuration
# -----------------------------------------------------------------------------
elif page == "Configuration":
    st.markdown("## Simulation Configuration")
    if not config:
        st.info("No configuration file found for this run.")
        st.stop()

    st.json(config)
    if "thresholds" in config:
        t = config["thresholds"]
        st.markdown("### Threshold Parameters")
        c1, c2, c3 = st.columns(3)
        c1.metric("E_critical", t.get("critical_energy", "N/A"))
        c2.metric("T_survival", t.get("survival_tribalism", "N/A"))
        c3.metric("C_hive", t.get("hive_collectivism", "N/A"))


# Footer.
st.markdown("---")
st.markdown(
    "<div style='text-align:center;color:#8b949e;font-size:0.8rem;'>"
    "CAT Simulation Engine v1.0.0 | Cosmobiological Asynchrony Theory | "
    "Apache License 2.0</div>",
    unsafe_allow_html=True,
)
