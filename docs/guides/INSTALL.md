# Installation Guide

This document describes the deterministic installation of the CAT Simulation Engine on Windows, macOS, and Linux. The engine consists of two independent subsystems — a Rust core that produces immutable run archives, and a Python analytics layer that consumes them — composed at the filesystem rather than coupled at the runtime.

A successful installation is defined as the ability to:

1. execute `cargo test --all-targets` with 35 / 35 tests passing;
2. execute `cargo run --release -- -t 10000 -n 2500 --seed 42` and produce a complete archive directory containing a valid `RUN_MANIFEST.json`;
3. launch `streamlit run dashboard.py` (or `python -m streamlit run dashboard.py` if that fails) and load the produced archive without error.

If any of these gates fails, the installation is incomplete. The [Troubleshooting](#5-troubleshooting) section covers the diagnostic procedures.

---

## Table of Contents

1. [System Requirements](#1-system-requirements)
2. [Windows Installation (PowerShell)](#2-windows-installation-powershell)
3. [Linux Installation (Debian / Ubuntu / Fedora / Arch)](#3-linux-installation-debian--ubuntu--fedora--arch)
4. [macOS Installation (Homebrew)](#4-macos-installation-homebrew)
5. [Troubleshooting](#5-troubleshooting)
6. [Verification Protocol](#6-verification-protocol)

---

## 1. System Requirements

### 1.1 Hardware

| Component | Minimum | Recommended |
|:---|:---|:---|
| CPU | 4 logical cores | 8+ logical cores (Rayon parallelism) |
| RAM | 4 GB | 16 GB (for $N \geq 10{,}000$ agents) |
| Disk | 2 GB free | 20 GB free (archive accumulation) |
| OS | Windows 10, macOS 12, Ubuntu 20.04 | Windows 11, macOS 14+, Ubuntu 22.04+ |

### 1.2 Software Toolchain

| Component | Version | Purpose |
|:---|:---|:---|
| **Rust** | Stable 1.78 or newer | Engine core |
| **Cargo** | Bundled with Rust | Build, test, dependency resolution |
| **rustfmt** | `rustup component add rustfmt` | Style gate |
| **clippy** | `rustup component add clippy` | Lint gate |
| **Python** | 3.12 or newer | Analytics layer |
| **pip** | Bundled with Python | Python dependency resolution |
| **venv** | Bundled with Python | Environment isolation |

### 1.3 Python Dependencies (declared in `analytics_python/requirements.txt`)

- **Streamlit** — Interactive dashboard runtime
- **Plotly** — Scientific visualization
- **Pandas** — Tabular data ingestion (`tick_history.csv`, `collapse_log.csv`, `final_agents.csv`)
- **NumPy** — Numerical primitives
- **SciPy** — Statistical post-processing
- **Matplotlib** — Static fallback rendering
- **Seaborn** — Distribution visualization

### 1.4 Rust Dependencies (declared in `engine_rust/Cargo.toml`)

- **Rayon** — Work-stealing parallelism for agent state advancement
- **Serde** + **serde_json** — Structured archive serialization
- **csv** — Tabular export
- **clap** — CLI argument parsing
- **rand** + **rand_chacha** — Deterministic seeded sampling
- **chrono** — Archive timestamping
- **uuid** (with `serde` feature) — Agent identity
- **env_logger** — Runtime diagnostics

---

## 2. Windows Installation (PowerShell)

The canonical project path contains a space (`CAT Model (Cosmobiological Asynchrony Theory)`). Every `cd` command in this section quotes its argument; unquoted invocations fail silently and produce non-obvious downstream errors.

### 2.1 Install Rust

```powershell
winget install --id Rustlang.Rustup -e
```

Reopen PowerShell, then:

```powershell
rustup default stable
rustup component add rustfmt clippy
rustc --version
cargo --version
```

Both version commands must print non-error output. If `cargo` is not found after a fresh install, see [§ 5.4](#54-rust-installed-but-cargo-is-not-found).

### 2.2 Install Python 3.12

```powershell
winget install --id Python.Python.3.12 -e
```

Reopen PowerShell, then:

```powershell
python --version
python -m pip --version
```

### 2.3 Build and Test the Rust Engine

```powershell
cd "C:\Users\Liran\Desktop\CAT Model (Cosmobiological Asynchrony Theory)\CAT-Simulation-Engine\engine_rust"
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo build --release
```

The test stage must report `test result: ok. 35 passed; 0 failed`.

### 2.4 Provision the Python Environment

```powershell
cd "C:\Users\Liran\Desktop\CAT Model (Cosmobiological Asynchrony Theory)\CAT-Simulation-Engine\analytics_python"
python -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
```

If PowerShell refuses to execute the activation script, see [§ 5.1](#51-powershell-cannot-run-the-virtual-environment-activation-script).

### 2.5 First End-to-End Run

```powershell
cd "..\engine_rust"
cargo run --release -- -t 10000 -n 2500 --seed 42

cd "..\analytics_python"
streamlit run dashboard.py
# If the above fails:
python -m streamlit run dashboard.py
```

The dashboard opens automatically at `http://localhost:8501`. Select the newly created archive under `data/runs/` and confirm that the population, state-vector, and collapse-cause panels render without error.

---

## 3. Linux Installation (Debian / Ubuntu / Fedora / Arch)

The commands below are written for **Debian / Ubuntu**. Equivalent invocations for Fedora and Arch are provided where they diverge.

### 3.1 Install System Packages

**Debian / Ubuntu:**

```bash
sudo apt update
sudo apt install -y build-essential curl pkg-config \
    python3.12 python3.12-venv python3-pip git
```

**Fedora:**

```bash
sudo dnf install -y gcc gcc-c++ make curl pkgconf-pkg-config \
    python3.12 python3-pip git
```

**Arch:**

```bash
sudo pacman -Syu --needed base-devel curl pkgconf python python-pip git
```

### 3.2 Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy
rustc --version
cargo --version
```

### 3.3 Build and Test the Rust Engine

```bash
cd "/path/to/CAT-Simulation-Engine/engine_rust"
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo build --release
```

The test stage must report `test result: ok. 35 passed; 0 failed`.

### 3.4 Provision the Python Environment

```bash
cd "../analytics_python"
python3.12 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
```

### 3.5 First End-to-End Run

```bash
cd ../engine_rust
cargo run --release -- -t 10000 -n 2500 --seed 42

cd ../analytics_python
streamlit run dashboard.py
# If the above fails:
python -m streamlit run dashboard.py
```

If the system browser does not open automatically, navigate manually to `http://localhost:8501`.

---

## 4. macOS Installation (Homebrew)

### 4.1 Install Homebrew (if absent)

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Follow the post-install instructions Homebrew prints — they may include adding Homebrew to your shell's PATH.

### 4.2 Install Rust and Python

```bash
brew install rustup-init python@3.12
rustup-init -y
source "$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy
rustc --version
cargo --version
python3.12 --version
```

### 4.3 Build and Test the Rust Engine

```bash
cd "/path/to/CAT-Simulation-Engine/engine_rust"
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo build --release
```

The test stage must report `test result: ok. 35 passed; 0 failed`.

### 4.4 Provision the Python Environment

```bash
cd "../analytics_python"
python3.12 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
```

### 4.5 First End-to-End Run

```bash
cd ../engine_rust
cargo run --release -- -t 10000 -n 2500 --seed 42

cd ../analytics_python
streamlit run dashboard.py
# If the above fails:
python -m streamlit run dashboard.py
```

On Apple Silicon (M1 / M2 / M3 / M4), Rust stable supports `aarch64-apple-darwin` natively; no Rosetta layer is required.

---

## 5. Troubleshooting

The failure modes documented below have been observed during the engine's development. Each is paired with the verified resolution.

### 5.1 PowerShell cannot run the virtual environment activation script

PowerShell's default execution policy blocks local scripts. Enable scripts for the current user only (this does not weaken machine-wide security):

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
.\.venv\Scripts\Activate.ps1
```

### 5.2 Commands fail when the project path contains spaces

The canonical project path contains parentheses and a space. Always quote it:

```powershell
cd "C:\Users\Liran\Desktop\CAT Model (Cosmobiological Asynchrony Theory)\CAT-Simulation-Engine"
```

This applies to every shell, including Bash on Windows Subsystem for Linux and PowerShell Core.

### 5.3 Cargo prints a path canonicalization warning on Windows

If Cargo emits `warning: could not canonicalize path C:\Users\Liran...`:

1. First confirm that `cargo check --all-targets` and `cargo test --all-targets` still complete with exit code 0. The warning is cosmetic in most cases.
2. The warning typically originates from path normalization around directories containing spaces or non-ASCII characters.
3. If the warning is intolerable for a reproducibility artifact, build from a shorter path:
   ```powershell
   Copy-Item -Recurse "C:\Users\Liran\Desktop\CAT Model (Cosmobiological Asynchrony Theory)\CAT-Simulation-Engine" "C:\tmp\CAT-Simulation-Engine"
   cd "C:\tmp\CAT-Simulation-Engine\engine_rust"
   cargo build --release
   ```

### 5.4 Rust installed but `cargo` is not found

`rustup` modifies the shell's PATH on install, but the modification does not propagate to already-open shell sessions. Either reopen the shell, or extend the PATH for the current session:

**PowerShell:**
```powershell
$env:Path += ";$env:USERPROFILE\.cargo\bin"
cargo --version
```

**Bash / Zsh (macOS / Linux):**
```bash
source "$HOME/.cargo/env"
cargo --version
```

### 5.5 Borrow checker errors after code changes

The simulation's six-phase architecture exists precisely to avoid aliasing mutable agent state. If a contribution introduces borrow checker failures:

1. Keep agent state advancement (Phase 2) independent — no agent reads another agent's mid-tick state.
2. Do not mutate the agent vector while iterating over borrowed agent entries.
3. Collect decisions first into an intermediate buffer, then apply mutations in a subsequent phase.
4. Keep random sampling on the main thread to preserve deterministic runs across thread counts.

Then re-validate:

```bash
cd engine_rust
cargo check --all-targets
```

The **first** compiler error is almost always the correct anchor point. Subsequent errors are typically downstream consequences of the same invalid ownership pattern.

### 5.6 Streamlit port conflict (8501 already in use)

```powershell
streamlit run dashboard.py --server.port 8502
# If the above fails:
python -m streamlit run dashboard.py --server.port 8502
```

Then open `http://localhost:8502`.

### 5.7 Dashboard shows "no completed runs"

The dashboard only loads run directories that contain a valid `RUN_MANIFEST.json`. Verify:

**PowerShell:**
```powershell
Get-ChildItem ..\data\runs -Directory
Get-ChildItem ..\data\runs\<run_directory>\RUN_MANIFEST.json
```

**Bash:**
```bash
ls -la ../data/runs/
ls -la ../data/runs/<run_directory>/RUN_MANIFEST.json
```

If the manifest is absent:

- The simulation was interrupted (SIGINT, OOM, crash) before the exporter completed.
- The output path was misconfigured. Re-run with an explicit `-o` / `--base-data-dir` flag.
- The run is in progress. Wait for completion before retrying the dashboard.

### 5.8 Path resolution: relative vs. absolute data directories

The engine resolves `-o` / `--base-data-dir` to an absolute path at startup. Relative paths are interpreted relative to the **current working directory at launch**, not relative to the engine binary. The engine then creates a timestamped subdirectory **inside** that base directory (`runs/YYYY-MM-DD_HHMMSS_seed<N>_n<M>/`), so pass the *base* path, not the `runs/` path. To avoid ambiguity, prefer absolute paths in production replications:

**PowerShell:**
```powershell
cargo run --release -- -t 10000 -n 2500 --seed 42 `
    -o "C:\Users\Liran\Desktop\CAT Model (Cosmobiological Asynchrony Theory)\CAT-Simulation-Engine\data"
```

**Bash:**
```bash
cargo run --release -- -t 10000 -n 2500 --seed 42 \
    -o "/absolute/path/to/CAT-Simulation-Engine/data"
```

### 5.9 `cargo test` reports `uuid` deserialization failure

The `uuid` crate's `serde` feature must be explicitly enabled in `Cargo.toml`. Confirm:

```toml
uuid = { version = "1", features = ["v4", "serde"] }
```

If the `serde` feature is missing, archive deserialization tests will fail. Add the feature, then:

```bash
cargo clean
cargo test --all-targets
```

### 5.10 Plotly figure crashes with "axis keys not recognized"

The analytics layer's `DARK_LAYOUT` deliberately omits axis configuration from the layout template. Axes are configured downstream via `fig.update_xaxes(...)` and `fig.update_yaxes(...)`. If a contributor inserts axis keys into the layout template directly, Plotly's stricter template API will reject them. The fix is to migrate the axis configuration out of the template and onto `update_*axes` calls.

### 5.11 Streamlit selectbox not reactive

If the run-selector dropdown does not trigger a re-render on selection, confirm that the widget is bound to `st.session_state` via an explicit `key`:

```python
st.selectbox("Select run", options=runs, key="run_selector")
```

Without an explicit `key`, Streamlit's diffing heuristic can fail to detect the change and the dashboard will appear frozen on the first-loaded run.

### 5.12 Numerical drift between runs with identical seeds

If two runs with identical `--seed`, `--ticks`, and `--agents` produce divergent `tick_history.csv` outputs:

1. Confirm both runs were executed in `--release` mode. Debug builds can produce different floating-point ordering on some platforms.
2. Confirm no contribution has introduced multi-threaded RNG sampling. All randomness flows through the main-thread `ChaCha8Rng`.
3. Confirm no contribution has replaced sorted agent iteration with HashMap iteration.

Determinism is a project-level invariant; a determinism failure is a release blocker.

---

## 6. Verification

Once installation is complete, run the verification sequence:

```bash
cd engine_rust
cargo test --all-targets
cargo run --release -- -t 10000 -n 2500 --seed 42
```

Inspect the produced archive.

PowerShell:
```powershell
Get-ChildItem ..\data\runs | Sort-Object LastWriteTime | Select-Object -Last 1
```

Bash:
```bash
ls -t ../data/runs | head -n 1
```

Confirm the archive contains `simulation_config.json`, `tick_history.json` and `tick_history.csv`, `collapse_log.json` and `collapse_log.csv`, `final_agents.csv`, at least one `snapshot_tick_NNNNNN.json`, and `RUN_MANIFEST.json`.

Then launch the dashboard:

```bash
cd ../analytics_python
streamlit run dashboard.py
# If the above fails:
python -m streamlit run dashboard.py
```

Select the newly created archive. The Asynchronous Gap collapse rate should appear in the 93 – 96 % range. If it does, the installation is verified. If not, return to [§ 5 Troubleshooting](#5-troubleshooting) before proceeding to research use.

---

*Last revision: 2026-05-15.*
