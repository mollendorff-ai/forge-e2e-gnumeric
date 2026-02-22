# Changelog

All notable changes to this project will be documented in this file.

## [1.1.1] - 2026-02-16

### Changed

- **Replace colored TUI with TAP v14 output**
  - Removed `colored` crate dependency
  - Output now uses TAP (Test Anything Protocol) version 14 format
  - YAML diagnostic blocks on failures (formula, expected, actual, error)
  - `# SKIP reason` directive for skipped tests
  - Diagnostic comments for config (forge path, engine version, test dir, mode)
  - Summary as TAP comment after results
  - Machine-parseable output enables CI/CD integration and downstream tooling

## [1.0.1] - 2026-01-24

### Changed

- **Rebranding: RoyalBit to Möllendorff AI**
  - Updated all references from RoyalBit to Möllendorff
  - **Why rebrand?** The "RoyalBit" name (company founded 2006) was hijacked by unrelated cryptocurrency scammers:
    - UK FCA issued official warning (Oct 2024) about "Royalbit Miners" - unauthorized firm
    - Multiple fraudulent domains: royalbit.ltd (trust score 38/100), royalbit.top, royal-bit.club
    - Classic HYIP Ponzi schemes offering impossible returns (155-580% in days)
    - Sources: [FCA Warning](https://www.fca.org.uk/news/warnings/royalbit-miners), [Scam Detector](https://www.scam-detector.com/validator/royalbit-ltd-review/)

## [1.0.0] - 2026-01-02

### Added

- Test runner with streaming and batch modes
- Gnumeric ssconvert integration for formula validation
- 1,909+ formula tests across 40 YAML files
- Function tests: math, text, date, financial, lookup, aggregation, statistical, trigonometric, logical, information
- Edge case tests: type coercion, error propagation, numeric precision
- Multi-sheet CSV parsing for flexible test organization
- Table data extraction for context-aware formula testing
