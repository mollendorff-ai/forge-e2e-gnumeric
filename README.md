# forge-e2e-gnumeric

E2E validation of [forge](https://github.com/royalbit/forge) against Gnumeric.

## Overview

Validates forge's Excel-compatible functions against Gnumeric (via `ssconvert`).
All tests are validated at runtime - no pre-computed expected values.

## Requirements

- **forge**: `FORGE_BIN` environment variable or `../forge/target/release/forge`
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
```

## How It Works

1. Load test YAML files with formulas and expected values
2. For each test, create minimal YAML with formula
3. Run `forge export` to create XLSX
4. Run `ssconvert --recalc` to recalculate formulas via Gnumeric
5. Parse CSV output and compare against expected value

## Test Structure

```
tests/
├── functions/    # Excel-compatible function tests
└── edge/         # Edge case tests
```

## License

Elastic License 2.0 - See [LICENSE](LICENSE)
