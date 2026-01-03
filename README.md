# forge-e2e-gnumeric

E2E validation of [Forge](https://github.com/royalbit/forge) Excel-compatible functions against Gnumeric.

## Overview

This test suite validates that Forge calculates Excel-compatible formulas correctly by comparing results against Gnumeric, an independent spreadsheet engine battle-tested since 2001.

**Core Philosophy**: Forge is NEVER the authority.
All validation comes from trusted third-party sources.

### Test Coverage

- **1,909+ formula tests** across 40 YAML files
- **30 function test files** covering math, text, date, financial, lookup, aggregation, statistical, trigonometric, logical, and information functions
- **10 edge case test files** covering boundary conditions, type coercion, error propagation, and numeric precision

## Requirements

- **Forge**: Set `FORGE_BIN` environment variable or place at `../forge/target/release/forge`
- **Gnumeric**: `ssconvert` in PATH
  ```bash
  # macOS
  brew install gnumeric

  # Ubuntu/Debian
  apt install gnumeric
  ```

## Usage

```bash
# Run all tests
FORGE_BIN=/path/to/forge cargo run --release -- --all

# Specify test directory
cargo run --release -- --tests tests/functions --all

# Batch mode (faster, single XLSX)
cargo run --release -- --all --batch
```

## How It Works

```
YAML Test Spec → forge export → XLSX → ssconvert --recalc → CSV → Compare
```

1. Load YAML test files with formulas and expected values
2. Create minimal YAML with the test formula
3. Run `forge export` to generate XLSX
4. Run `ssconvert --recalc` to recalculate via Gnumeric
5. Parse CSV output and compare against expected value

## Architecture

```
src/
├── main.rs      # CLI entry point
├── types.rs     # TestSpec, TestCase, TestResult structures
├── engine.rs    # Gnumeric ssconvert integration
├── runner.rs    # Test execution pipeline
└── excel.rs     # XLSX read/write helpers

tests/
├── functions/   # 30 YAML files - Excel function tests
└── edge/        # 10 YAML files - Edge case tests
```

## Test Format

```yaml
_forge_version: "1.0.0"

# Optional table data
sales:
  revenue: [100, 200, 300]
  cost: [50, 100, 150]

# Test cases
assumptions:
  test_sum_basic:
    formula: "=SUM(1, 2, 3)"
    expected: 6

  test_average_column:
    formula: "=AVERAGE(sales.revenue)"
    expected: 200
```

## Related Projects

- [forge](https://github.com/royalbit/forge) - Deterministic YAML-based financial modeling engine
- [forge-e2e](https://github.com/royalbit/forge-e2e) - E2E test suite documentation hub
- [forge-e2e-r](https://github.com/royalbit/forge-e2e-r) - Statistical validation against R

## License

Elastic License 2.0 - See [LICENSE](LICENSE)
