//! Gnumeric spreadsheet engine for formula recalculation.
//!
//! Uses ssconvert to recalculate Excel formulas and export to CSV.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Gnumeric spreadsheet engine for formula recalculation.
pub struct GnumericEngine {
    /// Path to the ssconvert binary.
    path: PathBuf,
    /// Version string from ssconvert.
    version: String,
}

impl GnumericEngine {
    /// Engine name constant.
    pub const NAME: &'static str = "Gnumeric (ssconvert)";

    /// Detects Gnumeric (ssconvert) installation.
    ///
    /// Returns `Some(engine)` if ssconvert is found and working,
    /// `None` otherwise.
    pub fn detect() -> Option<Self> {
        let output = Command::new("ssconvert").arg("--version").output().ok()?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Some(Self {
                path: PathBuf::from("ssconvert"),
                version,
            })
        } else {
            None
        }
    }

    /// Returns the engine version string.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the engine name.
    pub const fn name() -> &'static str {
        Self::NAME
    }

    /// Converts XLSX to CSV with formula recalculation.
    ///
    /// Uses ssconvert with the `--recalc` flag to ensure all formulas
    /// are recalculated before export.
    pub fn xlsx_to_csv(&self, xlsx_path: &Path, output_dir: &Path) -> Result<PathBuf, String> {
        let csv_name = xlsx_path
            .file_stem()
            .ok_or("Invalid xlsx path: no file stem")?
            .to_string_lossy()
            .to_string()
            + ".csv";
        let csv_path = output_dir.join(&csv_name);

        let output = Command::new(&self.path)
            .arg("--recalc")
            .arg(xlsx_path)
            .arg(&csv_path)
            .output()
            .map_err(|e| format!("Failed to run ssconvert: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "ssconvert failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        if csv_path.exists() {
            Ok(csv_path)
        } else {
            Err(format!("CSV file not created: {}", csv_path.display()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_name_is_constant() {
        assert_eq!(GnumericEngine::name(), "Gnumeric (ssconvert)");
    }

    #[test]
    fn engine_detection_returns_valid_engine_or_none() {
        let _ = GnumericEngine::detect();
    }
}
