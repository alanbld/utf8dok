//! Excel/XLSX data source using calamine.

use std::path::Path;

use calamine::{open_workbook, Data, Range, Reader, Xlsx, XlsxError};

use crate::error::{DataError, Result};
use crate::sources::DataSource;

/// Represents a parsed range specification
#[derive(Debug, Clone, PartialEq)]
pub enum RangeSpec {
    /// Standard cell range (A1:C10)
    CellRange {
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
    },
    /// Full column range (A:C) - uses all rows with data
    ColumnRange { start_col: u32, end_col: u32 },
    /// Full row range (1:10) - uses all columns with data
    RowRange { start_row: u32, end_row: u32 },
    /// Single cell (A1)
    SingleCell { row: u32, col: u32 },
    /// Use the entire used range of the sheet
    UsedRange,
}

/// Internal enum for parsing cell/column/row references
#[derive(Debug, Clone, PartialEq)]
enum CellOrRef {
    /// Full cell reference (A1) -> (col, row)
    Cell(u32, u32),
    /// Column only (A) -> col
    Column(u32),
    /// Row only (1) -> row
    Row(u32),
}

/// Options for cell value formatting
#[derive(Debug, Clone)]
pub struct CellFormatOptions {
    /// Attempt to detect and convert Excel date serial numbers to ISO format
    pub detect_dates: bool,
    /// Preserve error values (e.g., #DIV/0!) instead of converting to empty
    pub preserve_errors: bool,
    /// Number of decimal places for floats (None = auto)
    pub decimal_places: Option<u32>,
}

impl Default for CellFormatOptions {
    fn default() -> Self {
        Self {
            detect_dates: false, // Disabled by default to avoid false positives
            preserve_errors: true,
            decimal_places: None,
        }
    }
}

impl CellFormatOptions {
    /// Create options that detect dates automatically
    pub fn with_date_detection() -> Self {
        Self {
            detect_dates: true,
            ..Default::default()
        }
    }
}

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

    /// Parse a range string into a RangeSpec
    ///
    /// Supported formats:
    /// - "A1:C10" - Standard cell range
    /// - "A:C" - Full column range
    /// - "1:10" - Full row range
    /// - "A1" - Single cell
    /// - "*" or "" - Used range (all data)
    pub fn parse_range_spec(range: &str) -> Result<RangeSpec> {
        let range = range.trim();

        // Empty or "*" means used range
        if range.is_empty() || range == "*" {
            return Ok(RangeSpec::UsedRange);
        }

        // Check if it's a range (contains ':')
        if let Some(colon_pos) = range.find(':') {
            let left = &range[..colon_pos];
            let right = &range[colon_pos + 1..];

            let left_parsed = Self::parse_cell_or_ref(left)?;
            let right_parsed = Self::parse_cell_or_ref(right)?;

            match (left_parsed, right_parsed) {
                // Both are full cells (A1:C10)
                (CellOrRef::Cell(col1, row1), CellOrRef::Cell(col2, row2)) => {
                    Ok(RangeSpec::CellRange {
                        start_row: row1,
                        start_col: col1,
                        end_row: row2,
                        end_col: col2,
                    })
                }
                // Both are columns (A:C)
                (CellOrRef::Column(col1), CellOrRef::Column(col2)) => Ok(RangeSpec::ColumnRange {
                    start_col: col1,
                    end_col: col2,
                }),
                // Both are rows (1:10)
                (CellOrRef::Row(row1), CellOrRef::Row(row2)) => Ok(RangeSpec::RowRange {
                    start_row: row1,
                    end_row: row2,
                }),
                // Mixed types - not valid
                _ => Err(DataError::InvalidRange(format!(
                    "Mixed range types not supported: '{}'",
                    range
                ))),
            }
        } else {
            // Single reference - could be cell, column, or row
            match Self::parse_cell_or_ref(range)? {
                CellOrRef::Cell(col, row) => Ok(RangeSpec::SingleCell { row, col }),
                CellOrRef::Column(col) => Ok(RangeSpec::ColumnRange {
                    start_col: col,
                    end_col: col,
                }),
                CellOrRef::Row(row) => Ok(RangeSpec::RowRange {
                    start_row: row,
                    end_row: row,
                }),
            }
        }
    }

    /// Parse a range string like "A1:C10" into (start_row, start_col, end_row, end_col)
    /// Legacy method for backward compatibility
    fn parse_range(range: &str) -> Result<(u32, u32, u32, u32)> {
        match Self::parse_range_spec(range)? {
            RangeSpec::CellRange {
                start_row,
                start_col,
                end_row,
                end_col,
            } => Ok((start_row, start_col, end_row, end_col)),
            RangeSpec::SingleCell { row, col } => Ok((row, col, row, col)),
            _ => Err(DataError::InvalidRange(format!(
                "Range '{}' requires sheet dimensions; use read_range_extended",
                range
            ))),
        }
    }

    /// Parse a cell/column/row reference
    fn parse_cell_or_ref(s: &str) -> Result<CellOrRef> {
        let s = s.trim().to_uppercase();

        if s.is_empty() {
            return Err(DataError::InvalidRange("Empty reference".to_string()));
        }

        let mut col_str = String::new();
        let mut row_str = String::new();

        for c in s.chars() {
            if c.is_ascii_alphabetic() {
                if !row_str.is_empty() {
                    return Err(DataError::InvalidRange(format!(
                        "Invalid reference '{}': letters after numbers",
                        s
                    )));
                }
                col_str.push(c);
            } else if c.is_ascii_digit() {
                row_str.push(c);
            } else {
                return Err(DataError::InvalidRange(format!(
                    "Invalid character '{}' in reference",
                    c
                )));
            }
        }

        match (col_str.is_empty(), row_str.is_empty()) {
            // Both present: cell reference (A1)
            (false, false) => {
                let col = Self::column_to_index(&col_str)?;
                let row: u32 = row_str.parse().map_err(|_| {
                    DataError::InvalidRange(format!("Invalid row number '{}'", row_str))
                })?;
                if row == 0 {
                    return Err(DataError::InvalidRange(
                        "Row number must be >= 1".to_string(),
                    ));
                }
                Ok(CellOrRef::Cell(col, row - 1))
            }
            // Only column: column reference (A)
            (false, true) => {
                let col = Self::column_to_index(&col_str)?;
                Ok(CellOrRef::Column(col))
            }
            // Only row: row reference (1)
            (true, false) => {
                let row: u32 = row_str.parse().map_err(|_| {
                    DataError::InvalidRange(format!("Invalid row number '{}'", row_str))
                })?;
                if row == 0 {
                    return Err(DataError::InvalidRange(
                        "Row number must be >= 1".to_string(),
                    ));
                }
                Ok(CellOrRef::Row(row - 1))
            }
            // Neither: invalid
            (true, true) => Err(DataError::InvalidRange("Empty reference".to_string())),
        }
    }

    /// Parse a cell reference like "A1" into (column, row) as 0-indexed
    #[allow(dead_code)] // Used in tests
    fn parse_cell_ref(cell: &str) -> Result<(u32, u32)> {
        match Self::parse_cell_or_ref(cell)? {
            CellOrRef::Cell(col, row) => Ok((col, row)),
            _ => Err(DataError::InvalidRange(format!(
                "Expected cell reference, got '{}'",
                cell
            ))),
        }
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

    /// Convert 0-indexed column number to letters (0=A, 1=B, ..., 25=Z, 26=AA)
    pub fn index_to_column(mut index: u32) -> String {
        let mut result = String::new();
        loop {
            result.insert(0, (b'A' + (index % 26) as u8) as char);
            if index < 26 {
                break;
            }
            index = index / 26 - 1;
        }
        result
    }

    /// Convert a calamine cell to a string with formatting options
    #[allow(dead_code)] // Used in tests
    fn cell_to_string(cell: &Data) -> String {
        Self::cell_to_string_with_options(cell, &CellFormatOptions::default())
    }

    /// Convert a calamine cell to a string with custom formatting
    fn cell_to_string_with_options(cell: &Data, options: &CellFormatOptions) -> String {
        match cell {
            Data::Empty => String::new(),
            Data::String(s) => s.clone(),
            Data::Int(i) => i.to_string(),
            Data::Float(f) => {
                // Check if this might be an Excel date (serial number)
                if options.detect_dates && Self::looks_like_excel_date(*f) {
                    if let Some(date_str) = Self::excel_serial_to_iso(*f) {
                        return date_str;
                    }
                }
                // Format floats nicely (remove unnecessary decimals)
                if f.fract() == 0.0 {
                    format!("{:.0}", f)
                } else {
                    f.to_string()
                }
            }
            Data::Bool(b) => b.to_string(),
            Data::Error(e) => {
                if options.preserve_errors {
                    format!("#ERROR: {:?}", e)
                } else {
                    String::new()
                }
            }
            Data::DateTime(dt) => {
                // calamine DateTime wraps the serial number
                Self::excel_serial_to_iso(dt.as_f64()).unwrap_or_else(|| format!("{}", dt))
            }
            Data::DateTimeIso(s) => s.clone(),
            Data::DurationIso(s) => s.clone(),
        }
    }

    /// Check if a float value looks like an Excel date serial number
    fn looks_like_excel_date(value: f64) -> bool {
        // Excel dates are typically between 1 (1900-01-01) and ~55000 (2050)
        // We use a conservative range to avoid false positives
        (1.0..=55000.0).contains(&value) && value.fract() < 0.0001
    }

    /// Convert Excel serial date to ISO 8601 string
    ///
    /// Excel stores dates as days since 1899-12-30 (with a bug for 1900-02-29)
    fn excel_serial_to_iso(serial: f64) -> Option<String> {
        if serial < 1.0 {
            return None;
        }

        // Excel epoch is 1899-12-30, but there's a leap year bug
        // Dates >= 60 need adjustment for the fake 1900-02-29
        let days = if serial >= 60.0 {
            serial as i64 - 2 // Adjust for Excel's leap year bug
        } else {
            serial as i64 - 1
        };

        // Calculate date from days since 1900-01-01
        let base_year = 1900i32;
        let mut remaining_days = days;
        let mut year = base_year;

        loop {
            let days_in_year = if Self::is_leap_year(year) { 366 } else { 365 };
            if remaining_days < days_in_year as i64 {
                break;
            }
            remaining_days -= days_in_year as i64;
            year += 1;
        }

        let mut month = 1u32;
        loop {
            let days_in_month = Self::days_in_month(year, month);
            if remaining_days < days_in_month as i64 {
                break;
            }
            remaining_days -= days_in_month as i64;
            month += 1;
        }

        let day = remaining_days + 1;

        // Handle time component
        let time_frac = serial.fract();
        if time_frac > 0.0001 {
            let total_seconds = (time_frac * 86400.0).round() as u32;
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            Some(format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                year, month, day, hours, minutes, seconds
            ))
        } else {
            Some(format!("{:04}-{:02}-{:02}", year, month, day))
        }
    }

    fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::is_leap_year(year) {
                    29
                } else {
                    28
                }
            }
            _ => 30,
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
        Self::extract_range_data_with_options(
            sheet_range,
            start_row,
            start_col,
            end_row,
            end_col,
            &CellFormatOptions::default(),
        )
    }

    /// Extract data from a calamine Range with formatting options
    fn extract_range_data_with_options(
        sheet_range: &Range<Data>,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
        options: &CellFormatOptions,
    ) -> Result<Vec<Vec<String>>> {
        let mut result = Vec::new();

        for row_idx in start_row..=end_row {
            let mut row_data = Vec::new();
            for col_idx in start_col..=end_col {
                let cell = sheet_range.get((row_idx as usize, col_idx as usize));
                let value = match cell {
                    Some(data) => Self::cell_to_string_with_options(data, options),
                    None => String::new(),
                };
                row_data.push(value);
            }
            result.push(row_data);
        }

        Ok(result)
    }

    /// Read a range with extended syntax and formatting options
    ///
    /// Supports:
    /// - "A1:C10" - Standard cell range
    /// - "A:C" - Full columns (uses all rows with data)
    /// - "1:10" - Full rows (uses all columns with data)
    /// - "A1" - Single cell
    /// - "*" or "" - Entire used range
    pub fn read_range_extended(
        &self,
        sheet: &str,
        range: &str,
        options: &CellFormatOptions,
    ) -> Result<Vec<Vec<String>>> {
        let mut workbook: Xlsx<_> = open_workbook(&self.path)
            .map_err(|e| DataError::WorkbookOpen(format!("{}: {}", self.path, e)))?;

        let sheet_range = workbook
            .worksheet_range(sheet)
            .map_err(|e| DataError::SheetNotFound(format!("{}: {}", sheet, e)))?;

        let range_spec = Self::parse_range_spec(range)?;

        // Get sheet dimensions
        let (sheet_height, sheet_width) = sheet_range.get_size();
        let max_row = sheet_height.saturating_sub(1) as u32;
        let max_col = sheet_width.saturating_sub(1) as u32;

        // Resolve the range spec to concrete bounds
        let (start_row, start_col, end_row, end_col) = match range_spec {
            RangeSpec::CellRange {
                start_row,
                start_col,
                end_row,
                end_col,
            } => (start_row, start_col, end_row, end_col),

            RangeSpec::ColumnRange { start_col, end_col } => (0, start_col, max_row, end_col),

            RangeSpec::RowRange { start_row, end_row } => (start_row, 0, end_row, max_col),

            RangeSpec::SingleCell { row, col } => (row, col, row, col),

            RangeSpec::UsedRange => (0, 0, max_row, max_col),
        };

        Self::extract_range_data_with_options(
            &sheet_range,
            start_row,
            start_col,
            end_row,
            end_col,
            options,
        )
    }

    /// Read the entire used range of a sheet
    pub fn read_used_range(&self, sheet: &str) -> Result<Vec<Vec<String>>> {
        self.read_range_extended(sheet, "*", &CellFormatOptions::default())
    }

    /// Read the entire used range with date detection enabled
    pub fn read_used_range_with_dates(&self, sheet: &str) -> Result<Vec<Vec<String>>> {
        self.read_range_extended(sheet, "*", &CellFormatOptions::with_date_detection())
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
        // Single cell is now valid
        assert_eq!(ExcelSource::parse_range("A1").unwrap(), (0, 0, 0, 0));
        assert_eq!(ExcelSource::parse_range("B5").unwrap(), (4, 1, 4, 1));
    }

    #[test]
    fn test_parse_range_invalid() {
        // Multiple colons still invalid
        assert!(ExcelSource::parse_range("A1:B2:C3").is_err());
        // Column/row ranges require sheet dimensions (use read_range_extended)
        assert!(ExcelSource::parse_range("A:C").is_err());
        assert!(ExcelSource::parse_range("1:10").is_err());
        assert!(ExcelSource::parse_range("*").is_err());
    }

    #[test]
    fn test_parse_range_spec() {
        // Standard cell range
        assert_eq!(
            ExcelSource::parse_range_spec("A1:C10").unwrap(),
            RangeSpec::CellRange {
                start_row: 0,
                start_col: 0,
                end_row: 9,
                end_col: 2
            }
        );

        // Column range
        assert_eq!(
            ExcelSource::parse_range_spec("A:C").unwrap(),
            RangeSpec::ColumnRange {
                start_col: 0,
                end_col: 2
            }
        );

        // Single column
        assert_eq!(
            ExcelSource::parse_range_spec("B").unwrap(),
            RangeSpec::ColumnRange {
                start_col: 1,
                end_col: 1
            }
        );

        // Row range
        assert_eq!(
            ExcelSource::parse_range_spec("1:10").unwrap(),
            RangeSpec::RowRange {
                start_row: 0,
                end_row: 9
            }
        );

        // Single row
        assert_eq!(
            ExcelSource::parse_range_spec("5").unwrap(),
            RangeSpec::RowRange {
                start_row: 4,
                end_row: 4
            }
        );

        // Single cell
        assert_eq!(
            ExcelSource::parse_range_spec("B2").unwrap(),
            RangeSpec::SingleCell { row: 1, col: 1 }
        );

        // Used range
        assert_eq!(
            ExcelSource::parse_range_spec("*").unwrap(),
            RangeSpec::UsedRange
        );
        assert_eq!(
            ExcelSource::parse_range_spec("").unwrap(),
            RangeSpec::UsedRange
        );
    }

    #[test]
    fn test_parse_range_spec_invalid() {
        // Mixed types
        assert!(ExcelSource::parse_range_spec("A1:C").is_err());
        assert!(ExcelSource::parse_range_spec("A:10").is_err());
        assert!(ExcelSource::parse_range_spec("1:C").is_err());
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
    fn test_index_to_column() {
        assert_eq!(ExcelSource::index_to_column(0), "A");
        assert_eq!(ExcelSource::index_to_column(1), "B");
        assert_eq!(ExcelSource::index_to_column(25), "Z");
        assert_eq!(ExcelSource::index_to_column(26), "AA");
        assert_eq!(ExcelSource::index_to_column(27), "AB");
        assert_eq!(ExcelSource::index_to_column(51), "AZ");
        assert_eq!(ExcelSource::index_to_column(52), "BA");
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

    #[test]
    fn test_excel_serial_to_iso() {
        // 1 = 1900-01-01
        assert_eq!(
            ExcelSource::excel_serial_to_iso(1.0),
            Some("1900-01-01".to_string())
        );

        // 44927 = 2023-01-01 (approximately)
        let result = ExcelSource::excel_serial_to_iso(44927.0);
        assert!(result.is_some());
        assert!(result.unwrap().starts_with("2023-"));

        // Test with time component
        let result = ExcelSource::excel_serial_to_iso(44927.5); // noon
        assert!(result.is_some());
        assert!(result.unwrap().contains("T12:"));

        // Invalid (negative)
        assert_eq!(ExcelSource::excel_serial_to_iso(-1.0), None);
    }

    #[test]
    fn test_looks_like_excel_date() {
        // Valid date range
        assert!(ExcelSource::looks_like_excel_date(44927.0)); // 2023
        assert!(ExcelSource::looks_like_excel_date(1.0)); // 1900

        // Not dates (decimals)
        assert!(!ExcelSource::looks_like_excel_date(3.14));

        // Out of range
        assert!(!ExcelSource::looks_like_excel_date(0.5));
        assert!(!ExcelSource::looks_like_excel_date(100000.0));
    }
}
