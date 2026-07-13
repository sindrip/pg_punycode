use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

/// Workspace automation, invoked as `cargo xtask <command>`.
///
/// Every command first ensures that the cargo-pgrx CLI pinned by the
/// `[workspace.dependencies]` pgrx entry is installed in `.tools/`, so the CLI
/// and the pgrx crate the extension links against can never drift apart.
#[derive(Parser)]
#[command(name = "xtask")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Install the pinned cargo-pgrx into .tools/ (no-op if already current)
    Bootstrap,
    /// Run the extension test suite, e.g. `cargo xtask test pg18`
    Test {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Build + install the extension into the pgrx-managed Postgres and open psql
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Raw passthrough to the pinned cargo-pgrx, preserving the current directory,
    /// e.g. `cargo xtask pgrx -- init --pg18 download`
    Pgrx {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = workspace_root()?;
    let pgrx_bin = ensure_cargo_pgrx(&root)?;
    let ext_dir = root.join("crates/pg_punycode");

    let code = match cli.cmd {
        Cmd::Bootstrap => 0,
        Cmd::Test { args } => pgrx(&pgrx_bin, "test", &args, Some(&ext_dir))?,
        Cmd::Run { args } => pgrx(&pgrx_bin, "run", &args, Some(&ext_dir))?,
        Cmd::Pgrx { args } => {
            let (cmd, rest) = args
                .split_first()
                .context("usage: cargo xtask pgrx -- <command> [args]")?;
            pgrx(&pgrx_bin, cmd, rest, None)?
        }
    };
    std::process::exit(code);
}

fn workspace_root() -> Result<PathBuf> {
    // xtask always lives at <workspace root>/xtask
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("xtask crate has a parent directory")?
        .to_path_buf())
}

/// The exact pgrx version pinned in `[workspace.dependencies]` — the single
/// source of truth for both the extension crate and the cargo-pgrx CLI.
fn pinned_pgrx_version(root: &Path) -> Result<String> {
    let path = root.join("Cargo.toml");
    let manifest =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let doc: toml::Table = manifest.parse().context("parsing workspace Cargo.toml")?;
    let dep = doc
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.get("pgrx"))
        .context("no pgrx entry in [workspace.dependencies]")?;
    let req = match dep {
        toml::Value::String(s) => s.as_str(),
        toml::Value::Table(t) => t
            .get("version")
            .and_then(|v| v.as_str())
            .context("pgrx dependency table has no version field")?,
        _ => bail!("unexpected format for the pgrx workspace dependency"),
    };
    let version = req.trim_start_matches('=').trim();
    if !version.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        bail!("pgrx must be pinned to an exact version, found {req:?}");
    }
    Ok(version.to_string())
}

/// Ensure `.tools/bin/cargo-pgrx` exists at the pinned version, installing it
/// with `cargo install --root .tools` if missing or outdated.
fn ensure_cargo_pgrx(root: &Path) -> Result<PathBuf> {
    let version = pinned_pgrx_version(root)?;
    let tools = root.join(".tools");
    let bin = tools.join("bin").join("cargo-pgrx");

    if installed_version(&bin).as_deref() == Some(version.as_str()) {
        return Ok(bin);
    }

    eprintln!(
        "xtask: installing cargo-pgrx {version} into {}",
        tools.display()
    );
    let status = Command::new("cargo")
        .args(["install", "--locked", "cargo-pgrx", "--version"])
        .arg(format!("={version}"))
        .arg("--root")
        .arg(&tools)
        // this workspace's lint flags (e.g. CI's -D warnings) must not
        // apply to building a third-party tool; cargo install compiles the
        // root package without --cap-lints. Config-file rustflags still apply.
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .status()
        .context("running cargo install cargo-pgrx")?;
    if !status.success() {
        bail!("cargo install cargo-pgrx {version} failed");
    }

    match installed_version(&bin).as_deref() {
        Some(v) if v == version => Ok(bin),
        other => bail!("freshly installed cargo-pgrx reports {other:?}, expected {version}"),
    }
}

fn installed_version(bin: &Path) -> Option<String> {
    let out = Command::new(bin)
        .args(["pgrx", "--version"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    // output looks like "cargo-pgrx 0.19.1"
    String::from_utf8(out.stdout)
        .ok()?
        .split_whitespace()
        .last()
        .map(str::to_owned)
}

/// Run a cargo-pgrx subcommand. `dir: None` preserves the caller's cwd.
fn pgrx(bin: &Path, subcommand: &str, args: &[String], dir: Option<&Path>) -> Result<i32> {
    if let Some(d) = dir
        && !d.exists()
    {
        bail!(
            "{} does not exist yet — scaffold it with: cd crates && cargo xtask pgrx -- new pg_punycode",
            d.display()
        );
    }
    let mut cmd = Command::new(bin);
    cmd.arg("pgrx").arg(subcommand).args(args);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let status = cmd
        .status()
        .with_context(|| format!("running cargo-pgrx pgrx {subcommand}"))?;
    Ok(status.code().unwrap_or(1))
}
