//! Data source implementations.
//!
//! This module contains adapters for various data sources (Excel, CSV, etc.)

pub mod csv;
pub mod excel;

pub use csv::{CsvOptions, CsvSource};
pub use excel::{CellFormatOptions, ExcelSource, RangeSpec};

use crate::error::Result;

/// Trait for data sources that can provide tabular data
pub trait DataSource {
    /// Read a range of cells from the data source
    ///
    /// # Arguments
    /// * `sheet` - Sheet name (for multi-sheet sources like Excel)
    /// * `range` - Range specification (e.g., "A1:C10")
    ///
    /// # Returns
    /// A 2D vector of strings representing the cell values
    fn read_range(&self, sheet: &str, range: &str) -> Result<Vec<Vec<String>>>;

    /// List available sheets/tables in the source
    fn list_sheets(&self) -> Result<Vec<String>>;

    /// Get the default sheet name
    fn default_sheet(&self) -> Option<String>;
}
