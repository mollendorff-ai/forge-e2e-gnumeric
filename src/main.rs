//! forge-e2e-gnumeric: CLI entry point.
//!
//! Validates forge against Gnumeric (Excel-compatible functions).

use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use colored::Colorize;

use forge_e2e_gnumeric::engine::GnumericEngine;
use forge_e2e_gnumeric::runner::TestRunner;
use forge_e2e_gnumeric::types::TestResult;

#[derive(Parser)]
#[command(name = "forge-e2e-gnumeric")]
#[command(about = "E2E validation of forge against Gnumeric")]
#[command(version)]
struct Cli {
    /// Run all tests (headless mode with colored output).
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

    println!("{}", "forge-e2e-gnumeric".bold());
    println!("  Forge: {}", forge_binary.display());
    println!("  Engine: {} ({})", GnumericEngine::name(), engine.version());
    println!("  Tests: {}", cli.tests.display());
    println!();

    // Create runner
    let runner = TestRunner::new(forge_binary, engine, cli.tests)?;

    println!(
        "Loaded {} tests ({} skipped)",
        runner.test_cases().len(),
        runner.skip_cases().len()
    );
    println!();

    if cli.all {
        run_all_mode(&runner, cli.batch)?;
    } else {
        println!("Use --all to run all tests");
    }

    Ok(())
}

#[allow(clippy::unnecessary_wraps)] // Result for consistent main() error handling
fn run_all_mode(runner: &TestRunner, batch: bool) -> anyhow::Result<()> {
    let start = Instant::now();

    let results = if batch {
        println!("{}", "Running in batch mode...".cyan());
        runner.run_batch()
    } else {
        println!("{}", "Running tests...".cyan());
        runner.run_all_streaming(|result| {
            print_result(result);
        })
    };

    let elapsed = start.elapsed();

    // If batch mode, print results now
    if batch {
        for result in &results {
            print_result(result);
        }
    }

    // Summary
    println!();
    println!("{}", "═".repeat(60));

    let passed = results.iter().filter(|r| r.is_pass()).count();
    let failed = results.iter().filter(|r| r.is_fail()).count();
    let skipped = results.iter().filter(|r| matches!(r, TestResult::Skip { .. })).count();

    if failed == 0 {
        println!(
            "  {} {} passed, {} skipped in {:.2}s",
            "✓".green(),
            passed.to_string().green(),
            skipped,
            elapsed.as_secs_f64()
        );
    } else {
        println!(
            "  {} {} passed, {} failed, {} skipped in {:.2}s",
            "✗".red(),
            passed,
            failed.to_string().red(),
            skipped,
            elapsed.as_secs_f64()
        );
    }

    println!("{}", "═".repeat(60));

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn print_result(result: &TestResult) {
    match result {
        TestResult::Pass { name, .. } => {
            println!("  {} {}", "✓".green(), name);
        }
        TestResult::Fail { name, expected, actual, error, .. } => {
            println!("  {} {}", "✗".red(), name.red());
            if let Some(actual) = actual {
                println!("      expected: {expected}, actual: {actual}");
            }
            if let Some(error) = error {
                println!("      error: {error}");
            }
        }
        TestResult::Skip { name, reason } => {
            println!("  {} {} ({})", "○".yellow(), name.dimmed(), reason.dimmed());
        }
    }
}
