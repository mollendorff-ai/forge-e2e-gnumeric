//! forge-e2e-gnumeric: CLI entry point.
//!
//! Validates forge against Gnumeric (Excel-compatible functions).
//! Outputs results in TAP (Test Anything Protocol) version 14 format.

use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::Parser;

use forge_e2e_gnumeric::engine::GnumericEngine;
use forge_e2e_gnumeric::runner::TestRunner;
use forge_e2e_gnumeric::types::TestResult;

#[derive(Parser)]
#[command(name = "forge-e2e-gnumeric")]
#[command(about = "E2E validation of forge against Gnumeric")]
#[command(version)]
struct Cli {
    /// Run all tests.
    #[arg(long)]
    all: bool,

    /// Path to test specs directory.
    #[arg(short, long, default_value = "tests")]
    tests: PathBuf,

    /// Path to forge binary (or set `FORGE_BIN` env var).
    #[arg(short, long)]
    binary: Option<PathBuf>,

    /// Use batch mode (single XLSX, faster).
    #[arg(long)]
    batch: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Find forge binary
    let forge_binary = cli
        .binary
        .clone()
        .or_else(|| std::env::var("FORGE_BIN").ok().map(PathBuf::from))
        .or_else(|| {
            let relative = PathBuf::from("../forge/target/release/forge");
            if relative.exists() {
                Some(relative)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Forge binary not found. Set FORGE_BIN or use --binary, or build forge at ../forge/"
            )
        })?;

    if !forge_binary.exists() {
        anyhow::bail!("Forge binary not found: {}", forge_binary.display());
    }

    // Detect Gnumeric
    let engine = GnumericEngine::detect().ok_or_else(|| {
        anyhow::anyhow!(
            "Gnumeric (ssconvert) not found. Install with:\n  macOS: brew install gnumeric\n  Ubuntu: apt install gnumeric"
        )
    })?;

    // Create runner
    let runner = TestRunner::new(forge_binary.clone(), engine, cli.tests.clone())?;

    if cli.all {
        run_all(&cli, &runner, &forge_binary);
    } else {
        println!("# forge-e2e-gnumeric");
        println!("# Use --all to run all tests");
        println!(
            "# {} tests loaded ({} skipped)",
            runner.test_cases().len(),
            runner.skip_cases().len()
        );
    }

    Ok(())
}

fn run_all(cli: &Cli, runner: &TestRunner, forge_binary: &Path) {
    let total = runner.total_tests();
    let mode = if cli.batch { "batch" } else { "streaming" };

    // TAP header: diagnostic comments then version and plan
    println!("# forge-e2e-gnumeric");
    println!("# Forge: {}", forge_binary.display());
    println!(
        "# Engine: {} (ssconvert)",
        GnumericEngine::name()
    );
    println!("# Tests: {}", cli.tests.display());
    println!("# Mode: {mode}");
    println!("TAP version 14");
    println!("1..{total}");

    let start = Instant::now();

    let results = if cli.batch {
        let results = runner.run_batch();
        for (n, result) in results.iter().enumerate() {
            print_tap_line(n + 1, result);
        }
        results
    } else {
        let mut n: usize = 1;
        runner.run_all_streaming(|result| {
            print_tap_line(n, result);
            n += 1;
        })
    };

    let elapsed = start.elapsed();

    let passed = results.iter().filter(|r| r.is_pass()).count();
    let failed = results.iter().filter(|r| r.is_fail()).count();
    let skipped = results
        .iter()
        .filter(|r| matches!(r, TestResult::Skip { .. }))
        .count();

    println!(
        "# {passed} passed, {failed} failed, {skipped} skipped in {:.2}s",
        elapsed.as_secs_f64()
    );

    if failed > 0 {
        std::process::exit(1);
    }
}

fn print_tap_line(n: usize, result: &TestResult) {
    match result {
        TestResult::Pass { name, .. } => {
            println!("ok {n} - {name}");
        }
        TestResult::Fail {
            name,
            formula,
            expected,
            actual,
            error,
        } => {
            println!("not ok {n} - {name}");
            println!("  ---");
            println!("  formula: \"{formula}\"");
            println!("  expected: {expected}");
            if let Some(actual) = actual {
                println!("  actual: {actual}");
            }
            if let Some(error) = error {
                println!("  error: \"{error}\"");
            }
            println!("  ...");
        }
        TestResult::Skip { name, reason } => {
            println!("ok {n} - {name} # SKIP {reason}");
        }
    }
}
