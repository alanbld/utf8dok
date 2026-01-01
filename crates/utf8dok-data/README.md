# utf8dok-data

Data source integration for utf8dok. Converts Excel and CSV files to AsciiDoc tables.

## Features

- **Excel Support** - Read `.xlsx` and `.xls` files using calamine
- **CSV Support** - Read `.csv` and `.tsv` files with configurable options
- **Range Parsing** - Flexible range syntax (`A1:D10`, `A:C`, `1:100`, `*`)
- **Table Conversion** - Convert data to utf8dok AST tables

## Usage

### Read Excel File

```rust
use utf8dok_data::{ExcelSource, DataSource};

let source = ExcelSource::new("data.xlsx")?;
let data = source.read_range("Sheet1", "A1:D10")?;
```

### Read CSV File

```rust
use utf8dok_data::{CsvSource, CsvOptions};

let options = CsvOptions::default()
    .with_delimiter(b';');
let source = CsvSource::with_options("data.csv", options)?;
let data = source.read_range("data", "*")?;
```

### Convert to Table

```rust
use utf8dok_data::{TableConverter, ConvertOptions};

let options = ConvertOptions::with_header();
let table = TableConverter::convert(data, options);
```

## Range Syntax

| Syntax | Description |
|--------|-------------|
| `A1:D10` | Cell range from A1 to D10 |
| `A:C` | All rows in columns A through C |
| `1:100` | All columns in rows 1 through 100 |
| `A1` | Single cell |
| `*` | All data (used range) |

## CSV Options

```rust
CsvOptions::default()     // Comma-separated
CsvOptions::tsv()         // Tab-separated
CsvOptions::semicolon()   // Semicolon-separated
```

## Feature Flags

- `default` - Full functionality
- No optional features currently

## License

MIT OR Apache-2.0
