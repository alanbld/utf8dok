//! # utf8dok-data
//!
//! Data source integration for utf8dok - read Excel, CSV, and other tabular
//! data sources and convert them to AsciiDoc tables.
//!
//! ## Features
//!
//! - **Excel Support**: Read ranges from `.xlsx` files using `calamine`
//! - **AST Integration**: Convert tabular data to `utf8dok-ast` Table nodes
//! - **Range Parsing**: Standard Excel range notation (e.g., "A1:C10")
//!
//! ## Example
//!
//! ```rust,ignore
//! use utf8dok_data::{ExcelSource, DataSource, TableConverter, ConvertOptions};
//!
//! // Open an Excel file
//! let source = ExcelSource::new("data.xlsx")?;
//!
//! // Read a range
//! let data = source.read_range("Sheet1", "A1:C10")?;
//!
//! // Convert to AST Table
//! let table = TableConverter::convert_with_header(data);
//! ```

pub mod converter;
pub mod error;
pub mod sources;

// Re-exports
pub use converter::{ConvertOptions, TableConverter};
pub use error::{DataError, Result};
pub use sources::{CellFormatOptions, CsvOptions, CsvSource, DataSource, ExcelSource, RangeSpec};

/// Data engine for processing external data sources
pub struct DataEngine;

impl DataEngine {
    /// Read an Excel range and convert to an AST Table
    ///
    /// # Arguments
    /// * `path` - Path to the Excel file
    /// * `sheet` - Sheet name (optional, uses first sheet if None)
    /// * `range` - Cell range (e.g., "A1:C10")
    /// * `options` - Conversion options
    ///
    /// # Returns
    /// An AST Table block ready for document inclusion
    pub fn read_excel_table(
        path: &str,
        sheet: Option<&str>,
        range: &str,
        options: ConvertOptions,
    ) -> Result<utf8dok_ast::Table> {
        let source = ExcelSource::new(path)?;

        let sheet_name = match sheet {
            Some(s) => s.to_string(),
            None => source
                .default_sheet()
                .ok_or_else(|| DataError::SheetNotFound("No sheets in workbook".to_string()))?,
        };

        let data = source.read_range(&sheet_name, range)?;
        Ok(TableConverter::convert(data, options))
    }

    /// Read an Excel range with header row
    pub fn read_excel_table_with_header(
        path: &str,
        sheet: Option<&str>,
        range: &str,
    ) -> Result<utf8dok_ast::Table> {
        Self::read_excel_table(path, sheet, range, ConvertOptions::with_header())
    }

    /// Read a CSV file and convert to an AST Table
    ///
    /// # Arguments
    /// * `path` - Path to the CSV file
    /// * `range` - Row range (e.g., "1:10") or "*" for all rows
    /// * `options` - Conversion options
    /// * `csv_options` - CSV parsing options
    ///
    /// # Returns
    /// An AST Table block ready for document inclusion
    pub fn read_csv_table(
        path: &str,
        range: &str,
        options: ConvertOptions,
        csv_options: CsvOptions,
    ) -> Result<utf8dok_ast::Table> {
        let source = CsvSource::with_options(path, csv_options)?;
        let data = source.read_range("data", range)?;
        Ok(TableConverter::convert(data, options))
    }

    /// Read a CSV file with header row (using default CSV options)
    pub fn read_csv_table_with_header(path: &str, range: &str) -> Result<utf8dok_ast::Table> {
        Self::read_csv_table(
            path,
            range,
            ConvertOptions::with_header(),
            CsvOptions::default(),
        )
    }

    /// Read a TSV (tab-separated) file with header row
    pub fn read_tsv_table_with_header(path: &str, range: &str) -> Result<utf8dok_ast::Table> {
        Self::read_csv_table(
            path,
            range,
            ConvertOptions::with_header(),
            CsvOptions::tsv(),
        )
    }

    /// Auto-detect file type and read table
    ///
    /// Detects file type based on extension:
    /// - `.xlsx`, `.xls` → Excel
    /// - `.csv` → CSV (comma-separated)
    /// - `.tsv` → TSV (tab-separated)
    pub fn read_table_auto(
        path: &str,
        range: &str,
        options: ConvertOptions,
    ) -> Result<utf8dok_ast::Table> {
        let path_lower = path.to_lowercase();

        if path_lower.ends_with(".xlsx") || path_lower.ends_with(".xls") {
            Self::read_excel_table(path, None, range, options)
        } else if path_lower.ends_with(".tsv") {
            Self::read_csv_table(path, range, options, CsvOptions::tsv())
        } else {
            // Default to CSV for .csv and unknown extensions
            Self::read_csv_table(path, range, options, CsvOptions::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify all exports are accessible
        let _: fn(Vec<Vec<String>>, ConvertOptions) -> utf8dok_ast::Table = TableConverter::convert;
        let _: ConvertOptions = ConvertOptions::default();
    }
}
