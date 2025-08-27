//! Excel structure parsing for SPICE experiment files
//!
//! This module handles the parsing of Excel file structure, extracting column
//! mappings without making assumptions about tray names or specific layouts.

use anyhow::{Result, anyhow};
use calamine::Data;
use std::collections::HashMap;

/// Excel structure representation
#[derive(Debug)]
pub struct ExcelStructure {
    pub date_col: usize,
    pub time_col: usize,
    pub image_col: Option<usize>,
    pub well_columns: HashMap<String, usize>, // "TrayName:A1" -> column_index
    pub probe_columns: Vec<usize>,
    pub data_start_row: usize,
}

/// Parse Excel structure from raw rows
pub fn parse_excel_structure(rows: &[Vec<Data>]) -> Result<ExcelStructure> {
    if rows.len() < 7 {
        return Err(anyhow!("Excel file must have at least 7 rows"));
    }

    let tray_row = &rows[0];
    let coord_row = &rows[1];
    let header_row = &rows[6];

    let mut well_columns = HashMap::new();
    let mut probe_columns = Vec::new();
    let mut date_col = None;
    let mut time_col = None;
    let mut image_col = None;

    // Parse columns in a single pass
    for (col_idx, header_cell) in header_row.iter().enumerate() {
        if let Data::String(header) = header_cell {
            match header.as_str() {
                "Date" => date_col = Some(col_idx),
                "Time" => time_col = Some(col_idx),
                h if h.contains(".jpg") => image_col = Some(col_idx),
                h if h.starts_with("Temperature") => probe_columns.push(col_idx),
                "()" => {
                    // Well column - extract tray name and coordinate
                    if let Some(well_key) = extract_well_key(tray_row, coord_row, col_idx) {
                        well_columns.insert(well_key, col_idx);
                    }
                }
                _ => {} // Ignore other columns
            }
        }
    }

    Ok(ExcelStructure {
        date_col: date_col.ok_or_else(|| anyhow!("Missing Date column"))?,
        time_col: time_col.ok_or_else(|| anyhow!("Missing Time column"))?,
        image_col,
        well_columns,
        probe_columns,
        data_start_row: 7,
    })
}

/// Extract well key (tray:coordinate) from tray and coordinate rows
fn extract_well_key(tray_row: &[Data], coord_row: &[Data], col_idx: usize) -> Option<String> {
    let tray_name = extract_string_from_cell(tray_row.get(col_idx)?)?;
    let coordinate = extract_string_from_cell(coord_row.get(col_idx)?)?;

    // Basic validation - tray name shouldn't be empty and coordinate should look like A1, B2, etc.
    if !tray_name.trim().is_empty() && is_valid_coordinate(&coordinate) {
        Some(format!("{tray_name}:{coordinate}"))
    } else {
        None
    }
}

/// Extract string from Data cell, handling empty strings
fn extract_string_from_cell(cell: &Data) -> Option<String> {
    match cell {
        Data::String(s) if !s.trim().is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Validate coordinate format (e.g., "A1", "H12") without being overly strict
fn is_valid_coordinate(coord: &str) -> bool {
    if coord.len() < 2 {
        return false;
    }

    // Find where letters end and digits start
    let letter_end = coord.chars().take_while(char::is_ascii_alphabetic).count();
    if letter_end == 0 || letter_end == coord.len() {
        return false; // Must have both letters and digits
    }

    // Check that remaining characters are all digits
    coord.chars().skip(letter_end).all(|c| c.is_ascii_digit())
}

/// Parse well coordinate like "A1" into (`row_letter`, `column_number`)
pub fn parse_well_coordinate(coord: &str) -> Result<(String, i32)> {
    if coord.is_empty() {
        return Err(anyhow!("Empty coordinate"));
    }

    // Extract row letter(s) and column number
    let row_letter = coord
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect::<String>();
    let column_str = coord
        .chars()
        .skip_while(char::is_ascii_alphabetic)
        .collect::<String>();

    if row_letter.is_empty() || column_str.is_empty() {
        return Err(anyhow!("Invalid coordinate format: {coord}"));
    }

    let column_number = column_str
        .parse::<i32>()
        .map_err(|_| anyhow!("Invalid column number in coordinate: {coord}"))?;

    Ok((row_letter, column_number))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_excel_structure_parsing() {
        let test_data = vec![
            // Row 1: Tray names - any names should work
            vec![
                Data::String("Date".to_string()),
                Data::String("Time".to_string()),
                Data::String("NorthTray".to_string()),
                Data::String("SouthTray".to_string()),
            ],
            // Row 2: Well coordinates
            vec![
                Data::String(String::new()),
                Data::String(String::new()),
                Data::String("A1".to_string()),
                Data::String("B2".to_string()),
            ],
            // Rows 3-6: Empty
            vec![Data::String(String::new()); 4],
            vec![Data::String(String::new()); 4],
            vec![Data::String(String::new()); 4],
            vec![Data::String(String::new()); 4],
            // Row 7: Column headers
            vec![
                Data::String("Date".to_string()),
                Data::String("Time".to_string()),
                Data::String("()".to_string()),
                Data::String("()".to_string()),
            ],
        ];

        let result = parse_excel_structure(&test_data);
        assert!(result.is_ok(), "Should parse valid Excel structure");

        let structure = result.unwrap();
        assert_eq!(structure.date_col, 0);
        assert_eq!(structure.time_col, 1);
        assert_eq!(structure.well_columns.len(), 2);
        assert!(structure.well_columns.contains_key("NorthTray:A1"));
        assert!(structure.well_columns.contains_key("SouthTray:B2"));
        assert_eq!(structure.data_start_row, 7);
    }

    #[test]
    fn test_coordinate_validation() {
        assert!(is_valid_coordinate("A1"));
        assert!(is_valid_coordinate("H12"));
        assert!(is_valid_coordinate("AA1")); // Multi-letter rows should work
        assert!(is_valid_coordinate("Z99"));

        assert!(!is_valid_coordinate(""));
        assert!(!is_valid_coordinate("1A"));
        assert!(!is_valid_coordinate("A")); // No number
        assert!(!is_valid_coordinate("12")); // No letter
    }

    #[test]
    fn test_parse_well_coordinate() {
        assert_eq!(parse_well_coordinate("A1").unwrap(), ("A".to_string(), 1));
        assert_eq!(parse_well_coordinate("H12").unwrap(), ("H".to_string(), 12));
        assert_eq!(parse_well_coordinate("AA1").unwrap(), ("AA".to_string(), 1));

        assert!(parse_well_coordinate("").is_err());
        assert!(parse_well_coordinate("A").is_err());
        assert!(parse_well_coordinate("1").is_err());
    }

    #[test]
    fn test_extract_well_key() {
        let tray_row = vec![
            Data::String("TrayA".to_string()),
            Data::String("TrayB".to_string()),
            Data::String(String::new()), // Empty tray should be ignored
        ];
        let coord_row = vec![
            Data::String("A1".to_string()),
            Data::String("B2".to_string()),
            Data::String("C3".to_string()),
        ];

        assert_eq!(
            extract_well_key(&tray_row, &coord_row, 0),
            Some("TrayA:A1".to_string())
        );
        assert_eq!(
            extract_well_key(&tray_row, &coord_row, 1),
            Some("TrayB:B2".to_string())
        );
        assert_eq!(extract_well_key(&tray_row, &coord_row, 2), None); // Empty tray name
    }

    #[test]
    fn test_invalid_excel_format() {
        let insufficient_data = vec![
            vec![Data::String("P1".to_string())],
            vec![Data::String("A1".to_string())],
        ];

        let result = parse_excel_structure(&insufficient_data);
        assert!(result.is_err(), "Should reject insufficient data");
    }
}
