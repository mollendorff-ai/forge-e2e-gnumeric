//! Common types for forge-e2e-gnumeric.
//!
//! Defines the data structures for test specifications, test cases, and results.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Test specification file structure.
#[derive(Debug, Deserialize)]
pub struct TestSpec {
    /// The forge schema version (e.g., "1.0.0").
    #[serde(rename = "_forge_version")]
    pub forge_version: String,

    /// Named sections containing test definitions.
    #[serde(flatten)]
    pub sections: HashMap<String, Section>,
}

/// A section in the test spec (e.g., "assumptions", "projections").
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Section {
    /// A group of scalar values with optional formulas.
    ScalarGroup(HashMap<String, Scalar>),
    /// A table with columns of data.
    Table(HashMap<String, TableColumn>),
}

/// A scalar value with optional formula and expected value.
#[derive(Debug, Deserialize)]
pub struct Scalar {
    /// The literal value (if no formula).
    pub value: Option<f64>,
    /// The Excel formula to evaluate.
    pub formula: Option<String>,
    /// Expected value for E2E validation.
    pub expected: Option<f64>,
    /// Skip reason (if set, test is skipped).
    pub skip: Option<String>,
}

/// A table column (array of values or formula).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TableColumn {
    /// Column of numeric values.
    Numbers(Vec<f64>),
    /// Column of string values.
    Strings(Vec<String>),
    /// Column defined by a formula.
    Formula(String),
}

/// Individual test case extracted from a spec.
#[derive(Debug, Clone)]
pub struct TestCase {
    /// Fully qualified name (e.g., `assumptions.test_abs`).
    pub name: String,
    /// The Excel formula to evaluate.
    pub formula: String,
    /// The expected result value.
    pub expected: f64,
}

/// A test case that should be skipped.
#[derive(Debug, Clone)]
pub struct SkipCase {
    /// Fully qualified name.
    pub name: String,
    /// Reason for skipping.
    pub reason: String,
}

/// Result of running a test.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum TestResult {
    /// Test passed - actual matches expected.
    Pass {
        name: String,
        formula: String,
        expected: f64,
        actual: f64,
    },
    /// Test failed - mismatch or error.
    Fail {
        name: String,
        formula: String,
        expected: f64,
        actual: Option<f64>,
        error: Option<String>,
    },
    /// Test was skipped.
    Skip {
        name: String,
        reason: String,
    },
}

impl TestResult {
    /// Returns `true` if this result is a pass.
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }

    /// Returns `true` if this result is a failure.
    pub const fn is_fail(&self) -> bool {
        matches!(self, Self::Fail { .. })
    }

    /// Returns the test name.
    pub fn name(&self) -> &str {
        match self {
            Self::Pass { name, .. } | Self::Fail { name, .. } | Self::Skip { name, .. } => name,
        }
    }
}

/// Extracts test cases from a test spec.
///
/// Scans all sections for scalar values that have both a formula and
/// an expected value defined. Tests with `skip` field are excluded.
pub fn extract_test_cases(spec: &TestSpec) -> Vec<TestCase> {
    let mut cases = Vec::new();

    for (section_name, section) in &spec.sections {
        if section_name.starts_with('_') || section_name == "scenarios" {
            continue;
        }

        if let Section::ScalarGroup(scalars) = section {
            for (name, scalar) in scalars {
                // Skip tests marked with skip field
                if scalar.skip.is_some() {
                    continue;
                }
                if let (Some(formula), Some(expected)) = (&scalar.formula, scalar.expected) {
                    cases.push(TestCase {
                        name: format!("{section_name}.{name}"),
                        formula: formula.clone(),
                        expected,
                    });
                }
            }
        }
    }

    cases
}

/// Extracts skip cases from a test spec.
pub fn extract_skip_cases(spec: &TestSpec) -> Vec<SkipCase> {
    let mut cases = Vec::new();

    for (section_name, section) in &spec.sections {
        if section_name.starts_with('_') || section_name == "scenarios" {
            continue;
        }

        if let Section::ScalarGroup(scalars) = section {
            for (name, scalar) in scalars {
                if let Some(reason) = &scalar.skip {
                    cases.push(SkipCase {
                        name: format!("{section_name}.{name}"),
                        reason: reason.clone(),
                    });
                }
            }
        }
    }

    cases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_spec_extracts_test_cases() {
        let yaml = r#"
_forge_version: "1.0.0"
assumptions:
  test_abs:
    value: null
    formula: "=ABS(-42)"
    expected: 42
"#;
        let spec: TestSpec = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(spec.forge_version, "1.0.0");

        let cases = extract_test_cases(&spec);
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].name, "assumptions.test_abs");
    }

    #[test]
    fn test_result_is_pass() {
        let pass = TestResult::Pass {
            name: "test".to_string(),
            formula: "=1".to_string(),
            expected: 1.0,
            actual: 1.0,
        };
        assert!(pass.is_pass());
        assert!(!pass.is_fail());
    }
}
