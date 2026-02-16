//! Test runner - executes E2E validation tests against Gnumeric.
//!
//! Pipeline:
//! 1. Load test specs from YAML files
//! 2. For each test, generate a minimal YAML with the formula
//! 3. Run forge export to create XLSX
//! 4. Use Gnumeric (ssconvert) to recalculate and export to CSV
//! 5. Compare results against expected values

use std::fmt::Write;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::engine::GnumericEngine;
use crate::types::{
    extract_skip_cases, extract_table_data_yaml, extract_test_cases, SkipCase, TestCase,
    TestResult, TestSpec,
};

/// Test runner for E2E validation.
pub struct TestRunner {
    /// Path to the forge binary.
    forge_binary: PathBuf,
    /// Gnumeric engine for validation.
    engine: GnumericEngine,
    /// Directory containing test spec files.
    tests_dir: PathBuf,
    /// All loaded test cases.
    test_cases: Vec<TestCase>,
    /// All loaded skip cases.
    skip_cases: Vec<SkipCase>,
}

impl TestRunner {
    /// Creates a new test runner.
    ///
    /// # Errors
    ///
    /// Returns an error if the tests directory does not exist or YAML files
    /// cannot be read.
    pub fn new(
        forge_binary: PathBuf,
        engine: GnumericEngine,
        tests_dir: PathBuf,
    ) -> anyhow::Result<Self> {
        let (test_cases, skip_cases) = Self::load_test_cases(&tests_dir)?;

        Ok(Self {
            forge_binary,
            engine,
            tests_dir,
            test_cases,
            skip_cases,
        })
    }

    /// Finds all YAML files in a directory recursively.
    #[allow(dead_code)]
    fn find_yaml_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                files.extend(Self::find_yaml_files(&path)?);
            } else if path.extension().is_some_and(|e| e == "yaml") {
                files.push(path);
            }
        }
        Ok(files)
    }

    /// Loads all test cases from the tests directory.
    fn load_test_cases(tests_dir: &Path) -> anyhow::Result<(Vec<TestCase>, Vec<SkipCase>)> {
        let mut all_cases = Vec::new();
        let mut all_skips = Vec::new();

        if !tests_dir.exists() {
            anyhow::bail!("Tests directory does not exist: {}", tests_dir.display());
        }

        Self::load_test_cases_recursive(tests_dir, &mut all_cases, &mut all_skips)?;

        Ok((all_cases, all_skips))
    }

    fn load_test_cases_recursive(
        dir: &Path,
        all_cases: &mut Vec<TestCase>,
        all_skips: &mut Vec<SkipCase>,
    ) -> anyhow::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::load_test_cases_recursive(&path, all_cases, all_skips)?;
            } else if path.extension().is_some_and(|e| e == "yaml") {
                let content = fs::read_to_string(&path)?;
                match serde_yaml_ng::from_str::<TestSpec>(&content) {
                    Ok(spec) => {
                        let cases = extract_test_cases(&spec, Some(&path));
                        let skips = extract_skip_cases(&spec);
                        all_cases.extend(cases);
                        all_skips.extend(skips);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {e}", path.display());
                    }
                }
            }
        }

        Ok(())
    }

    /// Returns the total number of test cases.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Uses len() which isn't const
    pub fn total_tests(&self) -> usize {
        self.test_cases.len() + self.skip_cases.len()
    }

    #[allow(dead_code)]
    /// Returns test directory.
    #[must_use]
    pub const fn tests_dir(&self) -> &PathBuf {
        &self.tests_dir
    }

    /// Returns all test cases.
    #[must_use]
    pub fn test_cases(&self) -> &[TestCase] {
        &self.test_cases
    }

    /// Returns all skip cases.
    #[must_use]
    pub fn skip_cases(&self) -> &[SkipCase] {
        &self.skip_cases
    }

    /// Runs all tests and returns results.
    #[must_use]
    pub fn run_all(&self) -> Vec<TestResult> {
        self.skip_cases
            .iter()
            .map(|sc| TestResult::Skip {
                name: sc.name.clone(),
                reason: sc.reason.clone(),
            })
            .chain(self.test_cases.iter().map(|tc| self.run_test(tc)))
            .collect()
    }

    /// Runs all tests with streaming output via callback.
    pub fn run_all_streaming<F>(&self, mut on_result: F) -> Vec<TestResult>
    where
        F: FnMut(&TestResult),
    {
        let mut results = Vec::new();

        for skip_case in &self.skip_cases {
            let result = TestResult::Skip {
                name: skip_case.name.clone(),
                reason: skip_case.reason.clone(),
            };
            on_result(&result);
            results.push(result);
        }

        for tc in &self.test_cases {
            let result = self.run_test(tc);
            on_result(&result);
            results.push(result);
        }

        results
    }

    /// Runs all tests in batch mode (single XLSX, faster).
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn run_batch(&self) -> Vec<TestResult> {
        let mut results: Vec<TestResult> = self
            .skip_cases
            .iter()
            .map(|sc| TestResult::Skip {
                name: sc.name.clone(),
                reason: sc.reason.clone(),
            })
            .collect();

        if self.test_cases.is_empty() {
            return results;
        }

        // Create a single YAML with all test formulas
        let mut yaml_content = String::from("_forge_version: \"1.0.0\"\nassumptions:\n");
        for (i, tc) in self.test_cases.iter().enumerate() {
            let escaped_formula = tc.formula.replace('"', "\\\"");
            let _ = write!(
                yaml_content,
                "  test_{i}:\n    value: null\n    formula: \"{escaped_formula}\"\n"
            );
        }

        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                for tc in &self.test_cases {
                    results.push(TestResult::Fail {
                        name: tc.name.clone(),
                        formula: tc.formula.clone(),
                        expected: tc.expected,
                        actual: None,
                        error: Some(format!("Failed to create temp dir: {e}")),
                    });
                }
                return results;
            }
        };

        let yaml_path = temp_dir.path().join("batch.yaml");
        let xlsx_path = temp_dir.path().join("batch.xlsx");

        if let Err(e) = fs::write(&yaml_path, &yaml_content) {
            for tc in &self.test_cases {
                results.push(TestResult::Fail {
                    name: tc.name.clone(),
                    formula: tc.formula.clone(),
                    expected: tc.expected,
                    actual: None,
                    error: Some(format!("Failed to write YAML: {e}")),
                });
            }
            return results;
        }

        // Run forge export once
        let output = match Command::new(&self.forge_binary)
            .arg("export")
            .arg(&yaml_path)
            .arg(&xlsx_path)
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                for tc in &self.test_cases {
                    results.push(TestResult::Fail {
                        name: tc.name.clone(),
                        formula: tc.formula.clone(),
                        expected: tc.expected,
                        actual: None,
                        error: Some(format!("Failed to run forge: {e}")),
                    });
                }
                return results;
            }
        };

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            for tc in &self.test_cases {
                results.push(TestResult::Fail {
                    name: tc.name.clone(),
                    formula: tc.formula.clone(),
                    expected: tc.expected,
                    actual: None,
                    error: Some(format!("forge export failed: {err}")),
                });
            }
            return results;
        }

        // Convert XLSX to CSV using Gnumeric
        let csv_path = match self.engine.xlsx_to_csv(&xlsx_path, temp_dir.path()) {
            Ok(p) => p,
            Err(e) => {
                for tc in &self.test_cases {
                    results.push(TestResult::Fail {
                        name: tc.name.clone(),
                        formula: tc.formula.clone(),
                        expected: tc.expected,
                        actual: None,
                        error: Some(format!("CSV conversion failed: {e}")),
                    });
                }
                return results;
            }
        };

        // Parse CSV and match results
        let csv_results = Self::parse_batch_csv(&csv_path, self.test_cases.len());
        for (i, tc) in self.test_cases.iter().enumerate() {
            match csv_results.get(i) {
                Some(Ok(actual)) => {
                    if (*actual - tc.expected).abs() < f64::EPSILON {
                        results.push(TestResult::Pass {
                            name: tc.name.clone(),
                            formula: tc.formula.clone(),
                            expected: tc.expected,
                            actual: *actual,
                        });
                    } else {
                        results.push(TestResult::Fail {
                            name: tc.name.clone(),
                            formula: tc.formula.clone(),
                            expected: tc.expected,
                            actual: Some(*actual),
                            error: None,
                        });
                    }
                }
                Some(Err(e)) => {
                    results.push(TestResult::Fail {
                        name: tc.name.clone(),
                        formula: tc.formula.clone(),
                        expected: tc.expected,
                        actual: None,
                        error: Some(e.clone()),
                    });
                }
                None => {
                    results.push(TestResult::Fail {
                        name: tc.name.clone(),
                        formula: tc.formula.clone(),
                        expected: tc.expected,
                        actual: None,
                        error: Some("Missing result in CSV".to_string()),
                    });
                }
            }
        }

        results
    }

    fn parse_batch_csv(csv_path: &Path, count: usize) -> Vec<Result<f64, String>> {
        let mut results: Vec<Result<f64, String>> =
            vec![Err("Missing result in CSV output".to_string()); count];

        let file = match fs::File::open(csv_path) {
            Ok(f) => f,
            Err(e) => {
                for r in &mut results {
                    *r = Err(format!("Failed to open CSV: {e}"));
                }
                return results;
            }
        };

        let reader = BufReader::new(file);
        for line in reader.lines() {
            let Ok(line) = line else { continue };
            let cells: Vec<&str> = line
                .split(',')
                .map(|s| s.trim_matches('"').trim())
                .collect();

            if cells.len() >= 2 {
                let label = cells[0];
                if let Some(idx_str) = label
                    .strip_prefix("assumptions.test_")
                    .or_else(|| label.strip_prefix("test_"))
                {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        if idx < count {
                            if let Ok(value) = cells[1].replace(',', "").parse::<f64>() {
                                results[idx] = Ok(value);
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Runs a single test case.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn run_test(&self, test_case: &TestCase) -> TestResult {
        let escaped_formula = test_case.formula.replace('"', "\\\"");

        // Load table data from source file if available
        let table_data = if let Some(ref source_path) = test_case.source_file {
            if let Ok(content) = fs::read_to_string(source_path) {
                if let Ok(spec) = serde_yaml_ng::from_str::<TestSpec>(&content) {
                    extract_table_data_yaml(&spec)
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let forge_version = &test_case.forge_version;
        let yaml_content = format!(
            r#"_forge_version: "{forge_version}"
{table_data}assumptions:
  test_result:
    value: null
    formula: "{escaped_formula}"
"#
        );

        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                return TestResult::Fail {
                    name: test_case.name.clone(),
                    formula: test_case.formula.clone(),
                    expected: test_case.expected,
                    actual: None,
                    error: Some(format!("Failed to create temp dir: {e}")),
                };
            }
        };

        let yaml_path = temp_dir.path().join("test.yaml");
        let xlsx_path = temp_dir.path().join("test.xlsx");

        if let Err(e) = fs::write(&yaml_path, &yaml_content) {
            return TestResult::Fail {
                name: test_case.name.clone(),
                formula: test_case.formula.clone(),
                expected: test_case.expected,
                actual: None,
                error: Some(format!("Failed to write YAML: {e}")),
            };
        }

        // Run forge export
        let output = match Command::new(&self.forge_binary)
            .arg("export")
            .arg(&yaml_path)
            .arg(&xlsx_path)
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                return TestResult::Fail {
                    name: test_case.name.clone(),
                    formula: test_case.formula.clone(),
                    expected: test_case.expected,
                    actual: None,
                    error: Some(format!("Failed to run forge: {e}")),
                };
            }
        };

        if !output.status.success() {
            return TestResult::Fail {
                name: test_case.name.clone(),
                formula: test_case.formula.clone(),
                expected: test_case.expected,
                actual: None,
                error: Some(format!(
                    "forge export failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )),
            };
        }

        // Convert XLSX to CSV using Gnumeric (all sheets)
        let csv_files = match self
            .engine
            .xlsx_to_csv_all_sheets(&xlsx_path, temp_dir.path())
        {
            Ok(files) => files,
            Err(e) => {
                return TestResult::Fail {
                    name: test_case.name.clone(),
                    formula: test_case.formula.clone(),
                    expected: test_case.expected,
                    actual: None,
                    error: Some(format!("CSV conversion failed: {e}")),
                };
            }
        };

        // Search all sheets for the result
        for csv_path in &csv_files {
            if let Ok(actual) = Self::find_result_in_csv(csv_path, test_case.expected) {
                if (actual - test_case.expected).abs() < f64::EPSILON {
                    return TestResult::Pass {
                        name: test_case.name.clone(),
                        formula: test_case.formula.clone(),
                        expected: test_case.expected,
                        actual,
                    };
                }
                return TestResult::Fail {
                    name: test_case.name.clone(),
                    formula: test_case.formula.clone(),
                    expected: test_case.expected,
                    actual: Some(actual),
                    error: None,
                };
            }
        }

        TestResult::Fail {
            name: test_case.name.clone(),
            formula: test_case.formula.clone(),
            expected: test_case.expected,
            actual: None,
            error: Some("Could not find result in any CSV sheet".to_string()),
        }
    }

    fn find_result_in_csv(csv_path: &Path, expected: f64) -> Result<f64, String> {
        let file = fs::File::open(csv_path).map_err(|e| format!("Failed to open CSV: {e}"))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {e}"))?;
            let cells: Vec<&str> = line
                .split(',')
                .map(|s| s.trim_matches('"').trim())
                .collect();

            for (i, cell) in cells.iter().enumerate() {
                if (*cell == "result" || *cell == "test_result") && i + 1 < cells.len() {
                    if let Ok(value) = cells[i + 1].replace(',', "").parse::<f64>() {
                        return Ok(value);
                    }
                }

                if let Ok(value) = cell.replace(',', "").parse::<f64>() {
                    if (value - expected).abs() < 0.0001 {
                        return Ok(value);
                    }
                }
            }
        }

        Err("Could not find result in CSV output".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_empty_dir_returns_empty_cases() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = TestRunner::load_test_cases(temp_dir.path());
        assert!(result.is_ok());
        let (cases, skips) = result.unwrap();
        assert!(cases.is_empty());
        assert!(skips.is_empty());
    }

    #[test]
    fn load_nonexistent_dir_returns_error() {
        let result = TestRunner::load_test_cases(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn load_dir_with_yaml_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let yaml_content = r#"
_forge_version: "1.0.0"
assumptions:
  test_one:
    value: null
    formula: "=1+1"
    expected: 2
"#;
        fs::write(temp_dir.path().join("test.yaml"), yaml_content).unwrap();

        let result = TestRunner::load_test_cases(temp_dir.path());
        assert!(result.is_ok());
        let (cases, _) = result.unwrap();
        assert_eq!(cases.len(), 1);
    }
}
