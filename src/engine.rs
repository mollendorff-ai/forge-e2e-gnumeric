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
    #[must_use]
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
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the engine name.
    #[must_use]
    pub const fn name() -> &'static str {
        Self::NAME
    }

    /// Converts XLSX to CSV with formula recalculation.
    ///
    /// Uses ssconvert with the `--recalc` and `-S` flags to export all sheets
    /// as separate CSV files. Returns paths to all generated CSV files.
    ///
    /// # Errors
    ///
    /// Returns an error if the xlsx path has no file stem, ssconvert fails to
    /// run, or ssconvert exits with a non-zero status.
    pub fn xlsx_to_csv(&self, xlsx_path: &Path, output_dir: &Path) -> Result<PathBuf, String> {
        let base_name = xlsx_path
            .file_stem()
            .ok_or("Invalid xlsx path: no file stem")?
            .to_string_lossy()
            .to_string();

        // Export all sheets with -S flag, using %n for sheet number
        let csv_pattern = output_dir.join(format!("{base_name}_%n.csv"));

        let output = Command::new(&self.path)
            .arg("--recalc")
            .arg("-S")
            .arg(xlsx_path)
            .arg(&csv_pattern)
            .output()
            .map_err(|e| format!("Failed to run ssconvert: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "ssconvert failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Return pattern path - caller will handle finding the right sheet
        Ok(output_dir.join(format!("{base_name}_")))
    }

    /// Converts XLSX to CSV files (all sheets) and returns all CSV paths.
    ///
    /// # Errors
    ///
    /// Returns an error if the xlsx path has no file stem, ssconvert fails,
    /// or no CSV files are generated.
    pub fn xlsx_to_csv_all_sheets(
        &self,
        xlsx_path: &Path,
        output_dir: &Path,
    ) -> Result<Vec<PathBuf>, String> {
        let base_name = xlsx_path
            .file_stem()
            .ok_or("Invalid xlsx path: no file stem")?
            .to_string_lossy()
            .to_string();

        // Export all sheets with -S flag
        let csv_pattern = output_dir.join(format!("{base_name}_%n.csv"));

        let output = Command::new(&self.path)
            .arg("--recalc")
            .arg("-S")
            .arg(xlsx_path)
            .arg(&csv_pattern)
            .output()
            .map_err(|e| format!("Failed to run ssconvert: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "ssconvert failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Find all generated CSV files
        let mut csv_files = Vec::new();
        for i in 0..10 {
            // Check up to 10 sheets
            let csv_path = output_dir.join(format!("{base_name}_{i}.csv"));
            if csv_path.exists() {
                csv_files.push(csv_path);
            } else {
                break;
            }
        }

        if csv_files.is_empty() {
            Err("No CSV files generated".to_string())
        } else {
            Ok(csv_files)
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
