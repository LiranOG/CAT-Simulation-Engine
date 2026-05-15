# Python Analytics Layer

This directory contains the visualization and post-processing tools for CAT
simulation archives. The Rust engine writes structured run data; the Python
layer reads completed archives and turns them into interactive or static
research artifacts.

## Contents

| File | Purpose |
| --- | --- |
| `dashboard.py` | Streamlit application using Plotly to explore completed run archives interactively. |
| `logic_plotter.py` | Command-line static figure generator for publication-oriented PNG outputs. |
| `requirements.txt` | Python runtime dependencies for dashboarding, plotting, and data analysis. |

## Expected Input

The analytics layer expects a completed archive under:

```text
data/runs/YYYY-MM-DD_HHMMSS_seed<seed>_n<agents>/
```

A run is considered complete only when it contains `RUN_MANIFEST.json`. This
prevents the dashboard from silently loading partial output from interrupted
simulations.

## Dashboard Responsibilities

`dashboard.py` provides:

- Run discovery under a configurable base data directory.
- Warnings for incomplete archives and pre-archive orphaned data.
- Population dynamics charts.
- State-vector evolution charts for energy, tribalism, and collectivism.
- Collapse event analysis.
- Spatial distribution views.
- Asynchronous Gap visualization.
- Raw configuration and manifest inspection.

## Static Plotter Responsibilities

`logic_plotter.py` produces static figures from a single run directory. It is
intended for reports, papers, and offline analysis where deterministic image
files are preferable to an interactive dashboard.

## Operational Commands

```powershell
python -m pip install -r requirements.txt
streamlit run dashboard.py
# If the above fails:
python -m streamlit run dashboard.py
```

```powershell
python logic_plotter.py --data-dir ../data/runs/<run_directory>
```

## Maintenance Rules

- Keep all public function signatures typed.
- Keep every function documented with a docstring.
- Do not write simulation outputs from this layer; analytics should read and
  visualize archives produced by the Rust engine.
- Preserve the `RUN_MANIFEST.json` completion contract.
