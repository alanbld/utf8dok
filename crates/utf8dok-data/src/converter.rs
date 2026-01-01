//! Table converter - transforms raw data into AST Table nodes.

use utf8dok_ast::{Alignment, Block, ColumnSpec, Inline, Paragraph, Table, TableCell, TableRow};

/// Options for table conversion
#[derive(Debug, Clone, Default)]
pub struct ConvertOptions {
    /// Treat the first row as a header row
    pub first_row_header: bool,

    /// Default column alignment
    pub default_alignment: Option<Alignment>,

    /// Table style ID (for OOXML output)
    pub style_id: Option<String>,

    /// Table caption
    pub caption: Option<String>,
}

impl ConvertOptions {
    /// Create options with first row as header (most common case)
    pub fn with_header() -> Self {
        Self {
            first_row_header: true,
            ..Default::default()
        }
    }
}

/// Converts raw tabular data to AST Table nodes
pub struct TableConverter;

impl TableConverter {
    /// Convert raw 2D string data to an AST Table
    ///
    /// # Arguments
    /// * `data` - 2D vector of strings (rows × columns)
    /// * `options` - Conversion options
    ///
    /// # Returns
    /// An AST Table block
    pub fn convert(data: Vec<Vec<String>>, options: ConvertOptions) -> Table {
        if data.is_empty() {
            return Table {
                rows: Vec::new(),
                style_id: options.style_id,
                caption: options.caption.map(|c| vec![Inline::Text(c)]),
                columns: Vec::new(),
            };
        }

        let num_cols = data.iter().map(|row| row.len()).max().unwrap_or(0);

        // Generate column specs
        let columns: Vec<ColumnSpec> = (0..num_cols)
            .map(|_| ColumnSpec {
                width: None,
                align: options.default_alignment.clone(),
            })
            .collect();

        let mut rows = Vec::new();
        let mut data_iter = data.into_iter().peekable();

        // Handle header row
        if options.first_row_header {
            if let Some(header_data) = data_iter.next() {
                let header_row = Self::create_row(header_data, num_cols, true);
                rows.push(header_row);
            }
        }

        // Handle body rows
        for row_data in data_iter {
            let row = Self::create_row(row_data, num_cols, false);
            rows.push(row);
        }

        Table {
            rows,
            style_id: options.style_id,
            caption: options.caption.map(|c| vec![Inline::Text(c)]),
            columns,
        }
    }

    /// Create a table row from raw data
    fn create_row(data: Vec<String>, num_cols: usize, is_header: bool) -> TableRow {
        let mut cells = Vec::with_capacity(num_cols);

        for (i, value) in data.into_iter().enumerate() {
            if i >= num_cols {
                break;
            }
            cells.push(Self::create_cell(value));
        }

        // Pad with empty cells if needed
        while cells.len() < num_cols {
            cells.push(Self::create_cell(String::new()));
        }

        TableRow { cells, is_header }
    }

    /// Create a table cell from a string value
    fn create_cell(value: String) -> TableCell {
        let content = if value.is_empty() {
            Vec::new()
        } else {
            vec![Block::Paragraph(Paragraph {
                inlines: vec![Inline::Text(value)],
                style_id: None,
                attributes: std::collections::HashMap::new(),
            })]
        };

        TableCell {
            content,
            colspan: 1,
            rowspan: 1,
            align: None,
        }
    }

    /// Convert raw data to an AST Table with default options (first row as header)
    pub fn convert_with_header(data: Vec<Vec<String>>) -> Table {
        Self::convert(data, ConvertOptions::with_header())
    }

    /// Convert raw data to an AST Table without header
    pub fn convert_without_header(data: Vec<Vec<String>>) -> Table {
        Self::convert(data, ConvertOptions::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_empty() {
        let data: Vec<Vec<String>> = vec![];
        let table = TableConverter::convert(data, ConvertOptions::default());

        assert!(table.rows.is_empty());
        assert!(table.columns.is_empty());
    }

    #[test]
    fn test_convert_single_row_no_header() {
        let data = vec![vec!["A".to_string(), "B".to_string(), "C".to_string()]];
        let table = TableConverter::convert(data, ConvertOptions::default());

        assert_eq!(table.rows.len(), 1);
        assert!(!table.rows[0].is_header);
        assert_eq!(table.rows[0].cells.len(), 3);
        assert_eq!(table.columns.len(), 3);
    }

    #[test]
    fn test_convert_with_header() {
        let data = vec![
            vec!["Header1".to_string(), "Header2".to_string()],
            vec!["Value1".to_string(), "Value2".to_string()],
        ];
        let table = TableConverter::convert_with_header(data);

        assert_eq!(table.rows.len(), 2);
        assert!(table.rows[0].is_header);
        assert!(!table.rows[1].is_header);
    }

    #[test]
    fn test_convert_with_caption() {
        let data = vec![vec!["A".to_string()]];
        let options = ConvertOptions {
            caption: Some("My Table".to_string()),
            ..Default::default()
        };
        let table = TableConverter::convert(data, options);

        assert!(table.caption.is_some());
    }

    #[test]
    fn test_convert_uneven_rows() {
        let data = vec![
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            vec!["X".to_string()], // Shorter row
        ];
        let table = TableConverter::convert(data, ConvertOptions::default());

        assert_eq!(table.rows.len(), 2);
        // Second row should be padded to 3 columns
        assert_eq!(table.rows[1].cells.len(), 3);
    }

    #[test]
    fn test_cell_content() {
        let data = vec![vec!["Hello World".to_string()]];
        let table = TableConverter::convert(data, ConvertOptions::default());

        let cell = &table.rows[0].cells[0];
        assert_eq!(cell.content.len(), 1);

        if let Block::Paragraph(p) = &cell.content[0] {
            assert_eq!(p.inlines.len(), 1);
            if let Inline::Text(t) = &p.inlines[0] {
                assert_eq!(t, "Hello World");
            } else {
                panic!("Expected Text inline");
            }
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_empty_cell() {
        let data = vec![vec!["".to_string()]];
        let table = TableConverter::convert(data, ConvertOptions::default());

        let cell = &table.rows[0].cells[0];
        assert!(cell.content.is_empty());
    }
}
