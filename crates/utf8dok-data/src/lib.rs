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
pub use sources::{DataSource, ExcelSource};

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
