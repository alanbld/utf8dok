//! CSV data source.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::error::{DataError, Result};
use crate::sources::DataSource;

/// Options for CSV parsing
#[derive(Debug, Clone)]
pub struct CsvOptions {
    /// Field delimiter (default: comma)
    pub delimiter: u8,
    /// Whether the CSV has a header row
    pub has_header: bool,
    /// Quote character (default: double quote)
    pub quote: u8,
    /// Whether to trim whitespace from fields
    pub trim: bool,
    /// Whether to allow flexible column counts
    pub flexible: bool,
}

impl Default for CsvOptions {
    fn default() -> Self {
        Self {
            delimiter: b',',
            has_header: true,
            quote: b'"',
            trim: true,
            flexible: false,
        }
    }
}

impl CsvOptions {
    /// Create options for tab-separated values (TSV)
    pub fn tsv() -> Self {
        Self {
            delimiter: b'\t',
            ..Default::default()
        }
    }

    /// Create options for semicolon-separated values (common in European locales)
    pub fn semicolon() -> Self {
        Self {
            delimiter: b';',
            ..Default::default()
        }
    }

    /// Create options without header row
    pub fn without_header() -> Self {
        Self {
            has_header: false,
            ..Default::default()
        }
    }
}

/// CSV file data source
pub struct CsvSource {
    /// Path to the CSV file
    path: String,
    /// Parsing options
    options: CsvOptions,
    /// Cached row count (lazily computed)
    row_count: Option<usize>,
    /// Cached column count (lazily computed)
    col_count: Option<usize>,
}

impl CsvSource {
    /// Create a new CSV source from a file path
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Self::with_options(path, CsvOptions::default())
    }

    /// Create a new CSV source with custom options
    pub fn with_options(path: impl AsRef<Path>, options: CsvOptions) -> Result<Self> {
        let path_str = path.as_ref().display().to_string();

        if !path.as_ref().exists() {
            return Err(DataError::FileNotFound(path_str));
        }

        Ok(Self {
            path: path_str,
            options,
            row_count: None,
            col_count: None,
        })
    }

    /// Read all data from the CSV file
    pub fn read_all(&self) -> Result<Vec<Vec<String>>> {
        let file = File::open(&self.path).map_err(DataError::Io)?;
        let reader = BufReader::new(file);

        let mut csv_reader = csv::ReaderBuilder::new()
            .delimiter(self.options.delimiter)
            .quote(self.options.quote)
            .has_headers(false) // We handle headers ourselves
            .trim(if self.options.trim {
                csv::Trim::All
            } else {
                csv::Trim::None
            })
            .flexible(self.options.flexible)
            .from_reader(reader);

        let mut result = Vec::new();

        for record in csv_reader.records() {
            let record = record.map_err(|e| DataError::Calamine(e.to_string()))?;
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            result.push(row);
        }

        Ok(result)
    }

    /// Read a specific range of rows
    ///
    /// # Arguments
    /// * `start_row` - Starting row (0-indexed)
    /// * `end_row` - Ending row (inclusive, 0-indexed)
    pub fn read_rows(&self, start_row: usize, end_row: usize) -> Result<Vec<Vec<String>>> {
        let all_data = self.read_all()?;

        let end = end_row.min(all_data.len().saturating_sub(1));
        if start_row > end {
            return Ok(Vec::new());
        }

        Ok(all_data[start_row..=end].to_vec())
    }

    /// Read specific columns by index
    ///
    /// # Arguments
    /// * `columns` - Column indices to read (0-indexed)
    pub fn read_columns(&self, columns: &[usize]) -> Result<Vec<Vec<String>>> {
        let all_data = self.read_all()?;

        let result: Vec<Vec<String>> = all_data
            .into_iter()
            .map(|row| {
                columns
                    .iter()
                    .map(|&idx| row.get(idx).cloned().unwrap_or_default())
                    .collect()
            })
            .collect();

        Ok(result)
    }

    /// Get the number of rows in the CSV
    pub fn row_count(&mut self) -> Result<usize> {
        if let Some(count) = self.row_count {
            return Ok(count);
        }

        let data = self.read_all()?;
        let count = data.len();
        self.row_count = Some(count);
        Ok(count)
    }

    /// Get the number of columns (based on first row)
    pub fn col_count(&mut self) -> Result<usize> {
        if let Some(count) = self.col_count {
            return Ok(count);
        }

        let data = self.read_all()?;
        let count = data.first().map(|row| row.len()).unwrap_or(0);
        self.col_count = Some(count);
        Ok(count)
    }
}

impl DataSource for CsvSource {
    fn read_range(&self, _sheet: &str, range: &str) -> Result<Vec<Vec<String>>> {
        // CSV doesn't have sheets, ignore the sheet parameter
        // Parse range as row range (e.g., "1:10" means rows 1-10)

        let range = range.trim();

        // Empty range or "*" means all data
        if range.is_empty() || range == "*" {
            return self.read_all();
        }

        // Parse as row range (1:10)
        if let Some(colon_pos) = range.find(':') {
            let start = &range[..colon_pos];
            let end = &range[colon_pos + 1..];

            let start_row: usize = start
                .parse()
                .map_err(|_| DataError::InvalidRange(format!("Invalid start row: {}", start)))?;
            let end_row: usize = end
                .parse()
                .map_err(|_| DataError::InvalidRange(format!("Invalid end row: {}", end)))?;

            // Convert to 0-indexed
            if start_row == 0 || end_row == 0 {
                return Err(DataError::InvalidRange(
                    "Row numbers must be >= 1".to_string(),
                ));
            }

            return self.read_rows(start_row - 1, end_row - 1);
        }

        // Single row
        let row_num: usize = range
            .parse()
            .map_err(|_| DataError::InvalidRange(format!("Invalid row number: {}", range)))?;

        if row_num == 0 {
            return Err(DataError::InvalidRange(
                "Row number must be >= 1".to_string(),
            ));
        }

        self.read_rows(row_num - 1, row_num - 1)
    }

    fn list_sheets(&self) -> Result<Vec<String>> {
        // CSV files don't have sheets, return a single default sheet
        Ok(vec!["data".to_string()])
    }

    fn default_sheet(&self) -> Option<String> {
        Some("data".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_csv(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_csv_read_all() {
        let csv_content = "Name,Age,Score\nAlice,30,95\nBob,25,87\n";
        let file = create_test_csv(csv_content);

        let source = CsvSource::new(file.path()).unwrap();
        let data = source.read_all().unwrap();

        assert_eq!(data.len(), 3);
        assert_eq!(data[0], vec!["Name", "Age", "Score"]);
        assert_eq!(data[1], vec!["Alice", "30", "95"]);
        assert_eq!(data[2], vec!["Bob", "25", "87"]);
    }

    #[test]
    fn test_csv_read_range() {
        let csv_content = "Name,Age,Score\nAlice,30,95\nBob,25,87\nCharlie,35,92\n";
        let file = create_test_csv(csv_content);

        let source = CsvSource::new(file.path()).unwrap();

        // Read rows 2-3 (1-indexed)
        let data = source.read_range("data", "2:3").unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0], vec!["Alice", "30", "95"]);
        assert_eq!(data[1], vec!["Bob", "25", "87"]);
    }

    #[test]
    fn test_csv_read_all_via_range() {
        let csv_content = "A,B\n1,2\n";
        let file = create_test_csv(csv_content);

        let source = CsvSource::new(file.path()).unwrap();

        let data = source.read_range("data", "*").unwrap();
        assert_eq!(data.len(), 2);

        let data2 = source.read_range("data", "").unwrap();
        assert_eq!(data2.len(), 2);
    }

    #[test]
    fn test_csv_tsv() {
        let tsv_content = "Name\tAge\tScore\nAlice\t30\t95\n";
        let file = create_test_csv(tsv_content);

        let source = CsvSource::with_options(file.path(), CsvOptions::tsv()).unwrap();
        let data = source.read_all().unwrap();

        assert_eq!(data.len(), 2);
        assert_eq!(data[0], vec!["Name", "Age", "Score"]);
    }

    #[test]
    fn test_csv_semicolon() {
        let csv_content = "Name;Age;Score\nAlice;30;95\n";
        let file = create_test_csv(csv_content);

        let source = CsvSource::with_options(file.path(), CsvOptions::semicolon()).unwrap();
        let data = source.read_all().unwrap();

        assert_eq!(data.len(), 2);
        assert_eq!(data[0], vec!["Name", "Age", "Score"]);
    }

    #[test]
    fn test_csv_quoted_fields() {
        let csv_content = r#"Name,Description
"Alice","A ""quoted"" value"
"Bob","Value with, comma"
"#;
        let file = create_test_csv(csv_content);

        let source = CsvSource::new(file.path()).unwrap();
        let data = source.read_all().unwrap();

        assert_eq!(data.len(), 3);
        assert_eq!(data[1][1], r#"A "quoted" value"#);
        assert_eq!(data[2][1], "Value with, comma");
    }

    #[test]
    fn test_csv_list_sheets() {
        let csv_content = "A,B\n1,2\n";
        let file = create_test_csv(csv_content);

        let source = CsvSource::new(file.path()).unwrap();
        let sheets = source.list_sheets().unwrap();

        assert_eq!(sheets, vec!["data".to_string()]);
    }

    #[test]
    fn test_csv_default_sheet() {
        let csv_content = "A,B\n1,2\n";
        let file = create_test_csv(csv_content);

        let source = CsvSource::new(file.path()).unwrap();
        assert_eq!(source.default_sheet(), Some("data".to_string()));
    }

    #[test]
    fn test_csv_file_not_found() {
        let result = CsvSource::new("/nonexistent/path/file.csv");
        assert!(result.is_err());
    }

    #[test]
    fn test_csv_read_columns() {
        let csv_content = "A,B,C,D\n1,2,3,4\n5,6,7,8\n";
        let file = create_test_csv(csv_content);

        let source = CsvSource::new(file.path()).unwrap();
        let data = source.read_columns(&[0, 2]).unwrap();

        assert_eq!(data.len(), 3);
        assert_eq!(data[0], vec!["A", "C"]);
        assert_eq!(data[1], vec!["1", "3"]);
        assert_eq!(data[2], vec!["5", "7"]);
    }

    #[test]
    fn test_csv_trim_whitespace() {
        let csv_content = "Name , Age , Score \n Alice , 30 , 95 \n";
        let file = create_test_csv(csv_content);

        let source = CsvSource::new(file.path()).unwrap();
        let data = source.read_all().unwrap();

        assert_eq!(data[0], vec!["Name", "Age", "Score"]);
        assert_eq!(data[1], vec!["Alice", "30", "95"]);
    }
}
