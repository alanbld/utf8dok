//! Include directive parsing and resolution.
//!
//! This module handles `include::` directives for data files (Excel, CSV, TSV).
//!
//! # Syntax
//!
//! ```text
//! include::path/to/file.xlsx[sheet=Sheet1,range=A1:C10,header]
//! include::data.csv[range=1:100,header]
//! include::data.tsv[header]
//! ```
//!
//! # Attributes
//!
//! - `sheet=NAME` - Sheet name (Excel only, defaults to first sheet)
//! - `range=A1:C10` - Cell/row range (defaults to all data)
//! - `header` - Treat first row as header
//! - `delimiter=;` - Field delimiter (CSV only)
//!
//! # Example
//!
//! ```ignore
//! use utf8dok_core::include::{IncludeDirective, resolve_data_include};
//!
//! let directive = IncludeDirective::parse("include::data.xlsx[sheet=Sales,range=A1:D10,header]")?;
//! let table = resolve_data_include(&directive, ".")?;
//! ```

use std::collections::HashMap;
use std::path::Path;

use utf8dok_ast::Table;

/// Parsed include directive
#[derive(Debug, Clone, PartialEq)]
pub struct IncludeDirective {
    /// Path to the file
    pub path: String,
    /// Sheet name (for Excel files)
    pub sheet: Option<String>,
    /// Range specification (e.g., "A1:C10", "1:100", "*")
    pub range: Option<String>,
    /// Whether first row is a header
    pub header: bool,
    /// Field delimiter (for CSV files)
    pub delimiter: Option<char>,
    /// Additional attributes
    pub attributes: HashMap<String, String>,
}

impl IncludeDirective {
    /// Parse an include directive from a line
    ///
    /// # Format
    /// `include::path[attributes]`
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.trim();

        if !line.starts_with("include::") {
            return None;
        }

        let rest = &line[9..]; // Skip "include::"

        // Find brackets
        let bracket_start = rest.find('[')?;
        let bracket_end = rest.rfind(']')?;

        if bracket_start >= bracket_end {
            return None;
        }

        let path = rest[..bracket_start].trim().to_string();
        let attrs_str = &rest[bracket_start + 1..bracket_end];

        // Parse attributes
        let mut sheet = None;
        let mut range = None;
        let mut header = false;
        let mut delimiter = None;
        let mut attributes = HashMap::new();

        for attr in attrs_str.split(',') {
            let attr = attr.trim();

            if attr.is_empty() {
                continue;
            }

            if attr.eq_ignore_ascii_case("header") {
                header = true;
                continue;
            }

            if let Some(eq_pos) = attr.find('=') {
                let key = attr[..eq_pos].trim().to_lowercase();
                let value = attr[eq_pos + 1..].trim().to_string();

                match key.as_str() {
                    "sheet" => sheet = Some(value),
                    "range" => range = Some(value),
                    "delimiter" => delimiter = value.chars().next(),
                    _ => {
                        attributes.insert(key, value);
                    }
                }
            }
        }

        Some(Self {
            path,
            sheet,
            range,
            header,
            delimiter,
            attributes,
        })
    }

    /// Check if this is a data file include (Excel/CSV/TSV)
    pub fn is_data_file(&self) -> bool {
        let path_lower = self.path.to_lowercase();
        path_lower.ends_with(".xlsx")
            || path_lower.ends_with(".xls")
            || path_lower.ends_with(".csv")
            || path_lower.ends_with(".tsv")
    }

    /// Get the file extension
    pub fn extension(&self) -> Option<&str> {
        Path::new(&self.path).extension()?.to_str()
    }
}

/// Resolve a data include directive to a Table
///
/// # Arguments
/// * `directive` - The parsed include directive
/// * `base_path` - Base path for resolving relative paths
///
/// # Returns
/// A Table if successful, or an error message
#[cfg(feature = "data-includes")]
pub fn resolve_data_include(
    directive: &IncludeDirective,
    base_path: &str,
) -> Result<Table, String> {
    use utf8dok_data::{
        ConvertOptions, CsvOptions, CsvSource, DataSource, ExcelSource, TableConverter,
    };

    // Resolve the path
    let file_path = if Path::new(&directive.path).is_absolute() {
        directive.path.clone()
    } else {
        Path::new(base_path)
            .join(&directive.path)
            .to_string_lossy()
            .to_string()
    };

    // Check if file exists
    if !Path::new(&file_path).exists() {
        return Err(format!("Include file not found: {}", directive.path));
    }

    let ext = directive.extension().unwrap_or("").to_lowercase();
    let range = directive.range.as_deref().unwrap_or("*");

    // Load data based on file type
    let data: Vec<Vec<String>> = match ext.as_str() {
        "xlsx" | "xls" => {
            let source = ExcelSource::new(&file_path)
                .map_err(|e| format!("Failed to open Excel file: {}", e))?;

            let sheet = match &directive.sheet {
                Some(s) => s.clone(),
                None => source
                    .default_sheet()
                    .ok_or_else(|| "No sheets in workbook".to_string())?,
            };

            source
                .read_range(&sheet, range)
                .map_err(|e| format!("Failed to read Excel range: {}", e))?
        }
        "csv" => {
            let mut options = CsvOptions::default();
            if let Some(d) = directive.delimiter {
                options.delimiter = d as u8;
            }

            let source = CsvSource::with_options(&file_path, options)
                .map_err(|e| format!("Failed to open CSV file: {}", e))?;

            source
                .read_range("data", range)
                .map_err(|e| format!("Failed to read CSV: {}", e))?
        }
        "tsv" => {
            let source = CsvSource::with_options(&file_path, CsvOptions::tsv())
                .map_err(|e| format!("Failed to open TSV file: {}", e))?;

            source
                .read_range("data", range)
                .map_err(|e| format!("Failed to read TSV: {}", e))?
        }
        _ => {
            return Err(format!("Unsupported data file type: .{}", ext));
        }
    };

    // Convert to table
    let convert_options = if directive.header {
        ConvertOptions::with_header()
    } else {
        ConvertOptions::default()
    };

    Ok(TableConverter::convert(data, convert_options))
}

/// Stub resolver when data-includes feature is disabled
#[cfg(not(feature = "data-includes"))]
pub fn resolve_data_include(
    directive: &IncludeDirective,
    _base_path: &str,
) -> Result<Table, String> {
    Err(format!(
        "Data includes not supported (compile with 'data-includes' feature): {}",
        directive.path
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_include_basic() {
        let line = "include::data.xlsx[]";
        let directive = IncludeDirective::parse(line).unwrap();

        assert_eq!(directive.path, "data.xlsx");
        assert_eq!(directive.sheet, None);
        assert_eq!(directive.range, None);
        assert!(!directive.header);
    }

    #[test]
    fn test_parse_include_with_sheet() {
        let line = "include::report.xlsx[sheet=Sales]";
        let directive = IncludeDirective::parse(line).unwrap();

        assert_eq!(directive.path, "report.xlsx");
        assert_eq!(directive.sheet, Some("Sales".to_string()));
    }

    #[test]
    fn test_parse_include_with_range() {
        let line = "include::data.xlsx[range=A1:C10]";
        let directive = IncludeDirective::parse(line).unwrap();

        assert_eq!(directive.path, "data.xlsx");
        assert_eq!(directive.range, Some("A1:C10".to_string()));
    }

    #[test]
    fn test_parse_include_with_header() {
        let line = "include::data.csv[header]";
        let directive = IncludeDirective::parse(line).unwrap();

        assert_eq!(directive.path, "data.csv");
        assert!(directive.header);
    }

    #[test]
    fn test_parse_include_full() {
        let line = "include::path/to/data.xlsx[sheet=Sheet1,range=A1:D10,header]";
        let directive = IncludeDirective::parse(line).unwrap();

        assert_eq!(directive.path, "path/to/data.xlsx");
        assert_eq!(directive.sheet, Some("Sheet1".to_string()));
        assert_eq!(directive.range, Some("A1:D10".to_string()));
        assert!(directive.header);
    }

    #[test]
    fn test_parse_include_csv_delimiter() {
        let line = "include::data.csv[delimiter=;,header]";
        let directive = IncludeDirective::parse(line).unwrap();

        assert_eq!(directive.path, "data.csv");
        assert_eq!(directive.delimiter, Some(';'));
        assert!(directive.header);
    }

    #[test]
    fn test_parse_include_not_include() {
        assert!(IncludeDirective::parse("image::foo.png[]").is_none());
        assert!(IncludeDirective::parse("random text").is_none());
    }

    #[test]
    fn test_is_data_file() {
        let xlsx = IncludeDirective::parse("include::data.xlsx[]").unwrap();
        assert!(xlsx.is_data_file());

        let csv = IncludeDirective::parse("include::data.csv[]").unwrap();
        assert!(csv.is_data_file());

        let tsv = IncludeDirective::parse("include::data.tsv[]").unwrap();
        assert!(tsv.is_data_file());

        // Non-data files
        let adoc = IncludeDirective::parse("include::chapter.adoc[]").unwrap();
        assert!(!adoc.is_data_file());
    }

    #[test]
    fn test_extension() {
        let directive = IncludeDirective::parse("include::path/to/data.xlsx[]").unwrap();
        assert_eq!(directive.extension(), Some("xlsx"));

        let directive = IncludeDirective::parse("include::no_extension[]").unwrap();
        assert_eq!(directive.extension(), None);
    }
}
