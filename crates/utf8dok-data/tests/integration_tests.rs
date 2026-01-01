//! Integration tests for utf8dok-data

use std::path::PathBuf;

use utf8dok_data::{ConvertOptions, DataEngine, DataSource, ExcelSource, TableConverter};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn test_excel_source_new() {
    let path = fixture_path("test_data.xlsx");
    let source = ExcelSource::new(&path).expect("Failed to open Excel file");

    let sheets = source.list_sheets().expect("Failed to list sheets");
    assert_eq!(sheets.len(), 2);
    assert!(sheets.contains(&"TestData".to_string()));
    assert!(sheets.contains(&"Summary".to_string()));
}

#[test]
fn test_excel_source_default_sheet() {
    let path = fixture_path("test_data.xlsx");
    let source = ExcelSource::new(&path).expect("Failed to open Excel file");

    let default = source.default_sheet();
    assert_eq!(default, Some("TestData".to_string()));
}

#[test]
fn test_excel_read_range() {
    let path = fixture_path("test_data.xlsx");
    let source = ExcelSource::new(&path).expect("Failed to open Excel file");

    let data = source
        .read_range("TestData", "A1:C4")
        .expect("Failed to read range");

    assert_eq!(data.len(), 4); // 4 rows
    assert_eq!(data[0].len(), 3); // 3 columns

    // Check header row
    assert_eq!(data[0][0], "Name");
    assert_eq!(data[0][1], "Age");
    assert_eq!(data[0][2], "Score");

    // Check first data row
    assert_eq!(data[1][0], "Alice");
    assert_eq!(data[1][1], "30");
    assert_eq!(data[1][2], "95.5");

    // Check second data row
    assert_eq!(data[2][0], "Bob");
    assert_eq!(data[2][1], "25");
    assert_eq!(data[2][2], "87");
}

#[test]
fn test_excel_read_partial_range() {
    let path = fixture_path("test_data.xlsx");
    let source = ExcelSource::new(&path).expect("Failed to open Excel file");

    // Read only columns A and B, rows 1-2
    let data = source
        .read_range("TestData", "A1:B2")
        .expect("Failed to read range");

    assert_eq!(data.len(), 2);
    assert_eq!(data[0].len(), 2);
    assert_eq!(data[0][0], "Name");
    assert_eq!(data[0][1], "Age");
    assert_eq!(data[1][0], "Alice");
    assert_eq!(data[1][1], "30");
}

#[test]
fn test_excel_read_second_sheet() {
    let path = fixture_path("test_data.xlsx");
    let source = ExcelSource::new(&path).expect("Failed to open Excel file");

    let data = source
        .read_range("Summary", "A1:B2")
        .expect("Failed to read range");

    assert_eq!(data.len(), 2);
    assert_eq!(data[0][0], "Total");
    assert_eq!(data[0][1], "3");
    assert_eq!(data[1][0], "Average Score");
}

#[test]
fn test_excel_source_file_not_found() {
    let result = ExcelSource::new("/nonexistent/path/file.xlsx");
    assert!(result.is_err());
}

#[test]
fn test_table_converter_integration() {
    let path = fixture_path("test_data.xlsx");
    let source = ExcelSource::new(&path).expect("Failed to open Excel file");

    let data = source
        .read_range("TestData", "A1:C4")
        .expect("Failed to read range");

    let table = TableConverter::convert_with_header(data);

    // Verify table structure
    assert_eq!(table.rows.len(), 4);
    assert_eq!(table.columns.len(), 3);

    // First row should be header
    assert!(table.rows[0].is_header);
    assert!(!table.rows[1].is_header);
}

#[test]
fn test_data_engine_read_excel_table() {
    let path = fixture_path("test_data.xlsx");
    let path_str = path.to_str().unwrap();

    let table = DataEngine::read_excel_table_with_header(path_str, Some("TestData"), "A1:C4")
        .expect("Failed to read Excel table");

    assert_eq!(table.rows.len(), 4);
    assert!(table.rows[0].is_header);
    assert_eq!(table.columns.len(), 3);
}

#[test]
fn test_data_engine_default_sheet() {
    let path = fixture_path("test_data.xlsx");
    let path_str = path.to_str().unwrap();

    // Use None for sheet to test default selection
    let table =
        DataEngine::read_excel_table(path_str, None, "A1:C2", ConvertOptions::with_header())
            .expect("Failed to read Excel table");

    assert_eq!(table.rows.len(), 2);
}

#[test]
fn test_convert_options_style_id() {
    let path = fixture_path("test_data.xlsx");
    let source = ExcelSource::new(&path).expect("Failed to open Excel file");

    let data = source
        .read_range("TestData", "A1:C2")
        .expect("Failed to read range");

    let options = ConvertOptions {
        first_row_header: true,
        style_id: Some("TableGrid".to_string()),
        caption: Some("Test Table".to_string()),
        ..Default::default()
    };

    let table = TableConverter::convert(data, options);

    assert_eq!(table.style_id, Some("TableGrid".to_string()));
    assert!(table.caption.is_some());
}
