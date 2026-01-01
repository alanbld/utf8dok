//! Excel/XLSX data source using calamine.

use std::path::Path;

use calamine::{open_workbook, Data, Range, Reader, Xlsx, XlsxError};

use crate::error::{DataError, Result};
use crate::sources::DataSource;

/// Excel workbook data source
pub struct ExcelSource {
    /// Path to the Excel file
    path: String,
    /// Sheet names cache
    sheet_names: Vec<String>,
}

impl ExcelSource {
    /// Create a new Excel source from a file path
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path_str = path.as_ref().display().to_string();

        if !path.as_ref().exists() {
            return Err(DataError::FileNotFound(path_str));
        }

        let workbook: Xlsx<_> = open_workbook(path.as_ref())
            .map_err(|e: XlsxError| DataError::WorkbookOpen(e.to_string()))?;

        let sheet_names = workbook.sheet_names().to_vec();

        Ok(Self {
            path: path_str,
            sheet_names,
        })
    }

    /// Parse a range string like "A1:C10" into (start_row, start_col, end_row, end_col)
    fn parse_range(range: &str) -> Result<(u32, u32, u32, u32)> {
        let parts: Vec<&str> = range.split(':').collect();

        if parts.len() != 2 {
            return Err(DataError::InvalidRange(format!(
                "Expected format 'A1:B2', got '{}'",
                range
            )));
        }

        let (start_col, start_row) = Self::parse_cell_ref(parts[0])?;
        let (end_col, end_row) = Self::parse_cell_ref(parts[1])?;

        Ok((start_row, start_col, end_row, end_col))
    }

    /// Parse a cell reference like "A1" into (column, row) as 0-indexed
    fn parse_cell_ref(cell: &str) -> Result<(u32, u32)> {
        let cell = cell.trim().to_uppercase();

        if cell.is_empty() {
            return Err(DataError::InvalidRange("Empty cell reference".to_string()));
        }

        let mut col_str = String::new();
        let mut row_str = String::new();

        for c in cell.chars() {
            if c.is_ascii_alphabetic() {
                col_str.push(c);
            } else if c.is_ascii_digit() {
                row_str.push(c);
            } else {
                return Err(DataError::InvalidRange(format!(
                    "Invalid character '{}' in cell reference",
                    c
                )));
            }
        }

        if col_str.is_empty() || row_str.is_empty() {
            return Err(DataError::InvalidRange(format!(
                "Invalid cell reference '{}'",
                cell
            )));
        }

        // Convert column letters to 0-indexed number (A=0, B=1, ..., Z=25, AA=26, etc.)
        let col = Self::column_to_index(&col_str)?;

        // Convert row to 0-indexed
        let row: u32 = row_str
            .parse::<u32>()
            .map_err(|_| DataError::InvalidRange(format!("Invalid row number '{}'", row_str)))?;

        if row == 0 {
            return Err(DataError::InvalidRange(
                "Row number must be >= 1".to_string(),
            ));
        }

        Ok((col, row - 1)) // Convert to 0-indexed
    }

    /// Convert column letters to 0-indexed number (A=0, B=1, ..., Z=25, AA=26)
    fn column_to_index(col: &str) -> Result<u32> {
        let mut result: u32 = 0;
        for c in col.chars() {
            let value = c as u32 - 'A' as u32 + 1;
            result = result * 26 + value;
        }
        Ok(result - 1) // 0-indexed
    }

    /// Convert a calamine cell to a string
    fn cell_to_string(cell: &Data) -> String {
        match cell {
            Data::Empty => String::new(),
            Data::String(s) => s.clone(),
            Data::Int(i) => i.to_string(),
            Data::Float(f) => {
                // Format floats nicely (remove unnecessary decimals)
                if f.fract() == 0.0 {
                    format!("{:.0}", f)
                } else {
                    f.to_string()
                }
            }
            Data::Bool(b) => b.to_string(),
            Data::Error(e) => format!("#ERROR: {:?}", e),
            Data::DateTime(dt) => format!("{}", dt),
            Data::DateTimeIso(s) => s.clone(),
            Data::DurationIso(s) => s.clone(),
        }
    }

    /// Extract data from a calamine Range
    fn extract_range_data(
        sheet_range: &Range<Data>,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
    ) -> Result<Vec<Vec<String>>> {
        let mut result = Vec::new();

        for row_idx in start_row..=end_row {
            let mut row_data = Vec::new();
            for col_idx in start_col..=end_col {
                let cell = sheet_range.get((row_idx as usize, col_idx as usize));
                let value = match cell {
                    Some(data) => Self::cell_to_string(data),
                    None => String::new(),
                };
                row_data.push(value);
            }
            result.push(row_data);
        }

        Ok(result)
    }
}

impl DataSource for ExcelSource {
    fn read_range(&self, sheet: &str, range: &str) -> Result<Vec<Vec<String>>> {
        // Re-open workbook for reading (calamine requires this pattern)
        let mut workbook: Xlsx<_> = open_workbook(&self.path)
            .map_err(|e| DataError::WorkbookOpen(format!("{}: {}", self.path, e)))?;

        // Get the sheet range
        let sheet_range = workbook
            .worksheet_range(sheet)
            .map_err(|e| DataError::SheetNotFound(format!("{}: {}", sheet, e)))?;

        // Parse the range specification
        let (start_row, start_col, end_row, end_col) = Self::parse_range(range)?;

        // Extract data
        Self::extract_range_data(&sheet_range, start_row, start_col, end_row, end_col)
    }

    fn list_sheets(&self) -> Result<Vec<String>> {
        Ok(self.sheet_names.clone())
    }

    fn default_sheet(&self) -> Option<String> {
        self.sheet_names.first().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cell_ref() {
        assert_eq!(ExcelSource::parse_cell_ref("A1").unwrap(), (0, 0));
        assert_eq!(ExcelSource::parse_cell_ref("B2").unwrap(), (1, 1));
        assert_eq!(ExcelSource::parse_cell_ref("Z1").unwrap(), (25, 0));
        assert_eq!(ExcelSource::parse_cell_ref("AA1").unwrap(), (26, 0));
        assert_eq!(ExcelSource::parse_cell_ref("AB10").unwrap(), (27, 9));
    }

    #[test]
    fn test_parse_cell_ref_case_insensitive() {
        assert_eq!(ExcelSource::parse_cell_ref("a1").unwrap(), (0, 0));
        assert_eq!(ExcelSource::parse_cell_ref("b2").unwrap(), (1, 1));
    }

    #[test]
    fn test_parse_range() {
        // Returns (start_row, start_col, end_row, end_col)
        assert_eq!(ExcelSource::parse_range("A1:B2").unwrap(), (0, 0, 1, 1));
        assert_eq!(ExcelSource::parse_range("A1:C10").unwrap(), (0, 0, 9, 2));
    }

    #[test]
    fn test_parse_range_invalid() {
        assert!(ExcelSource::parse_range("A1").is_err());
        assert!(ExcelSource::parse_range("A1:B2:C3").is_err());
        assert!(ExcelSource::parse_range("").is_err());
    }

    #[test]
    fn test_column_to_index() {
        assert_eq!(ExcelSource::column_to_index("A").unwrap(), 0);
        assert_eq!(ExcelSource::column_to_index("B").unwrap(), 1);
        assert_eq!(ExcelSource::column_to_index("Z").unwrap(), 25);
        assert_eq!(ExcelSource::column_to_index("AA").unwrap(), 26);
        assert_eq!(ExcelSource::column_to_index("AB").unwrap(), 27);
        assert_eq!(ExcelSource::column_to_index("AZ").unwrap(), 51);
        assert_eq!(ExcelSource::column_to_index("BA").unwrap(), 52);
    }

    #[test]
    fn test_cell_to_string() {
        assert_eq!(ExcelSource::cell_to_string(&Data::Empty), "");
        assert_eq!(
            ExcelSource::cell_to_string(&Data::String("hello".to_string())),
            "hello"
        );
        assert_eq!(ExcelSource::cell_to_string(&Data::Int(42)), "42");
        assert_eq!(ExcelSource::cell_to_string(&Data::Float(3.14)), "3.14");
        assert_eq!(ExcelSource::cell_to_string(&Data::Float(10.0)), "10");
        assert_eq!(ExcelSource::cell_to_string(&Data::Bool(true)), "true");
    }
}
