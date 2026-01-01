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

// ==================== EDGE CASE TESTS ====================

#[test]
fn test_excel_invalid_sheet_name() {
    let path = fixture_path("test_data.xlsx");
    let source = ExcelSource::new(&path).expect("Failed to open Excel file");

    let result = source.read_range("NonexistentSheet", "A1:C4");
    assert!(result.is_err());
}

#[test]
fn test_excel_out_of_bounds_range() {
    let path = fixture_path("test_data.xlsx");
    let source = ExcelSource::new(&path).expect("Failed to open Excel file");

    // Request a range beyond the data
    let data = source.read_range("TestData", "A1:Z100");
    // Should return data up to the actual extent
    assert!(data.is_ok());
}

#[test]
fn test_table_converter_empty_data() {
    let data: Vec<Vec<String>> = vec![];
    let table = TableConverter::convert(data, ConvertOptions::default());

    assert_eq!(table.rows.len(), 0);
    assert_eq!(table.columns.len(), 0);
}

#[test]
fn test_table_converter_single_row() {
    let data = vec![vec!["Header1".to_string(), "Header2".to_string()]];
    let table = TableConverter::convert_with_header(data);

    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.columns.len(), 2);
    assert!(table.rows[0].is_header);
}

#[test]
fn test_table_converter_single_cell() {
    let data = vec![vec!["OnlyCell".to_string()]];
    let table = TableConverter::convert(data, ConvertOptions::default());

    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.columns.len(), 1);
    assert_eq!(table.rows[0].cells[0].content.len(), 1);
}

#[test]
fn test_data_engine_auto_detect_xlsx() {
    let path = fixture_path("test_data.xlsx");
    let path_str = path.to_str().unwrap();

    let table = DataEngine::read_table_auto(path_str, "A1:C2", ConvertOptions::default())
        .expect("Failed to auto-detect and read xlsx");

    assert_eq!(table.rows.len(), 2);
}

#[test]
fn test_excel_read_all_data() {
    use utf8dok_data::CellFormatOptions;

    let path = fixture_path("test_data.xlsx");
    let source = ExcelSource::new(&path).expect("Failed to open Excel file");

    // Use "*" to read all data (requires extended range)
    let data = source
        .read_range_extended("TestData", "*", &CellFormatOptions::default())
        .expect("Failed to read all data");

    // Should have all rows
    assert!(!data.is_empty());
    // First row should be header
    assert!(data[0].contains(&"Name".to_string()));
}

#[test]
fn test_csv_read_basic() {
    use utf8dok_data::CsvSource;

    // Create a temporary CSV file
    let temp_dir = std::env::temp_dir();
    let csv_path = temp_dir.join("test_basic.csv");
    std::fs::write(&csv_path, "Name,Age,Score\nAlice,30,95.5\nBob,25,87.0\n")
        .expect("Failed to write CSV");

    let source = CsvSource::new(&csv_path).expect("Failed to open CSV");
    let data = source.read_range("data", "*").expect("Failed to read CSV");

    assert_eq!(data.len(), 3); // header + 2 rows
    assert_eq!(data[0][0], "Name");
    assert_eq!(data[1][0], "Alice");

    // Cleanup
    std::fs::remove_file(csv_path).ok();
}

#[test]
fn test_csv_with_semicolon_delimiter() {
    use utf8dok_data::{CsvOptions, CsvSource};

    let temp_dir = std::env::temp_dir();
    let csv_path = temp_dir.join("test_semicolon.csv");
    std::fs::write(&csv_path, "Name;Age;Score\nAlice;30;95.5\n").expect("Failed to write CSV");

    let source =
        CsvSource::with_options(&csv_path, CsvOptions::semicolon()).expect("Failed to open CSV");
    let data = source.read_range("data", "*").expect("Failed to read CSV");

    assert_eq!(data.len(), 2);
    assert_eq!(data[0][0], "Name");
    assert_eq!(data[0][2], "Score");

    std::fs::remove_file(csv_path).ok();
}

#[test]
fn test_csv_row_range() {
    use utf8dok_data::CsvSource;

    let temp_dir = std::env::temp_dir();
    let csv_path = temp_dir.join("test_row_range.csv");
    std::fs::write(&csv_path, "A,B,C\n1,2,3\n4,5,6\n7,8,9\n10,11,12\n")
        .expect("Failed to write CSV");

    let source = CsvSource::new(&csv_path).expect("Failed to open CSV");

    // Read rows 2-4 (0-indexed)
    let data = source
        .read_range("data", "2:4")
        .expect("Failed to read CSV");

    // Should have rows 2, 3, 4 (1,2,3 / 4,5,6 / 7,8,9)
    assert_eq!(data.len(), 3);
    assert_eq!(data[0][0], "1");

    std::fs::remove_file(csv_path).ok();
}

#[test]
fn test_csv_unicode_data() {
    use utf8dok_data::CsvSource;

    let temp_dir = std::env::temp_dir();
    let csv_path = temp_dir.join("test_unicode.csv");
    std::fs::write(
        &csv_path,
        "Name,City,Emoji\nHéléne,Zürich,\u{1F600}\nТест,Москва,\u{1F389}\n日本語,東京,\u{1F3EF}\n",
    )
    .expect("Failed to write CSV");

    let source = CsvSource::new(&csv_path).expect("Failed to open CSV");
    let data = source.read_range("data", "*").expect("Failed to read CSV");

    assert_eq!(data.len(), 4); // header + 3 rows
    assert_eq!(data[1][0], "Héléne");
    assert_eq!(data[2][0], "Тест");
    assert_eq!(data[3][0], "日本語");

    std::fs::remove_file(csv_path).ok();
}

#[test]
fn test_csv_empty_cells() {
    use utf8dok_data::CsvSource;

    let temp_dir = std::env::temp_dir();
    let csv_path = temp_dir.join("test_empty.csv");
    std::fs::write(&csv_path, "A,B,C\n1,,3\n,2,\n,,\n").expect("Failed to write CSV");

    let source = CsvSource::new(&csv_path).expect("Failed to open CSV");
    let data = source.read_range("data", "*").expect("Failed to read CSV");

    assert_eq!(data.len(), 4);
    assert_eq!(data[1][0], "1");
    assert_eq!(data[1][1], ""); // empty cell
    assert_eq!(data[1][2], "3");

    std::fs::remove_file(csv_path).ok();
}

#[test]
fn test_tsv_file() {
    use utf8dok_data::{CsvOptions, CsvSource};

    let temp_dir = std::env::temp_dir();
    let tsv_path = temp_dir.join("test_file.tsv");
    std::fs::write(&tsv_path, "Name\tAge\tScore\nAlice\t30\t95.5\n").expect("Failed to write TSV");

    let source = CsvSource::with_options(&tsv_path, CsvOptions::tsv()).expect("Failed to open TSV");
    let data = source.read_range("data", "*").expect("Failed to read TSV");

    assert_eq!(data.len(), 2);
    assert_eq!(data[0][0], "Name");
    assert_eq!(data[0][1], "Age");

    std::fs::remove_file(tsv_path).ok();
}
