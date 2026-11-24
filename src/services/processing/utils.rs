//! Utility functions for Excel data processing
//!
//! This module contains helper functions for data extraction, timestamp parsing,
//! and other common operations used during Excel processing.

use anyhow::{Result, anyhow};
use calamine::Data;
use chrono::{NaiveDateTime, Utc};
use rust_decimal::Decimal;

use super::structure::ExcelStructure;

/// Extract decimal value from Excel cell data
pub fn extract_decimal(cell: &Data) -> Option<Decimal> {
    match cell {
        Data::Float(f) => Decimal::from_f64_retain(*f),
        Data::Int(i) => Some(Decimal::from(*i)),
        _ => None,
    }
}

/// Extract integer value from Excel cell data
pub fn extract_integer(cell: &Data) -> Option<i32> {
    match cell {
        Data::Int(i) => i32::try_from(*i).ok(),
        Data::Float(f) => {
            let rounded = f.round();
            // Check if finite and within i32 range
            if rounded.is_finite()
                && rounded >= f64::from(i32::MIN)
                && rounded <= f64::from(i32::MAX)
            {
                // Safe cast: checked bounds and finiteness above
                #[allow(clippy::cast_possible_truncation)]
                Some(rounded as i32)
            } else {
                None // Return None for out-of-range or non-finite values
            }
        }
        _ => None,
    }
}

/// Parse timestamp from Excel row data
pub fn parse_timestamp(row: &[Data], structure: &ExcelStructure) -> Result<chrono::DateTime<Utc>> {
    let date_cell = row
        .get(structure.date_col)
        .ok_or_else(|| anyhow!("Missing date"))?;
    let time_cell = row
        .get(structure.time_col)
        .ok_or_else(|| anyhow!("Missing time"))?;

    match (date_cell, time_cell) {
        (Data::String(date_str), Data::String(time_str)) => {
            let combined = format!("{date_str} {time_str}");

            // Try multiple datetime formats
            if let Ok(dt) = NaiveDateTime::parse_from_str(&combined, "%Y-%m-%d %H:%M:%S") {
                Ok(dt.and_utc())
            } else if let Ok(dt) = NaiveDateTime::parse_from_str(&combined, "%m/%d/%Y %H:%M:%S") {
                Ok(dt.and_utc())
            } else if let Ok(dt) = NaiveDateTime::parse_from_str(&combined, "%Y-%m-%d %H:%M:%S%.f")
            {
                Ok(dt.and_utc())
            } else if let Ok(dt) = NaiveDateTime::parse_from_str(&combined, "%m/%d/%Y %H:%M:%S%.f")
            {
                Ok(dt.and_utc())
            } else {
                Err(anyhow!("Could not parse datetime: {combined}"))
            }
        }
        (Data::DateTime(excel_dt), _) => {
            // Use calamine's Excel date as float and convert
            let timestamp_secs = (excel_dt.as_f64() - 25569.0) * 86400.0; // Excel epoch to Unix epoch

            // Check bounds more precisely for f64 -> i64 conversion
            if timestamp_secs.is_finite() {
                // Safe cast: checked that value is finite above
                #[allow(clippy::cast_possible_truncation)]
                let timestamp_int = timestamp_secs as i64;
                Ok(chrono::DateTime::from_timestamp(timestamp_int, 0)
                    .ok_or_else(|| anyhow!("Invalid timestamp: {}", timestamp_secs))?)
            } else {
                Err(anyhow!("Excel timestamp is not finite: {}", timestamp_secs))
            }
        }
        (Data::Float(timestamp), _) => {
            // Handle float timestamp as Unix timestamp
            let rounded_timestamp = timestamp.round();

            // Check if finite before converting
            if rounded_timestamp.is_finite() {
                // Safe cast: checked that value is finite above
                #[allow(clippy::cast_possible_truncation)]
                let timestamp_int = rounded_timestamp as i64;
                Ok(chrono::DateTime::from_timestamp(timestamp_int, 0)
                    .ok_or_else(|| anyhow!("Invalid timestamp: {}", rounded_timestamp))?
                    .with_timezone(&chrono::Utc))
            } else {
                Err(anyhow!(
                    "Float timestamp is not finite: {}",
                    rounded_timestamp
                ))
            }
        }
        _ => Err(anyhow!(
            "Unsupported timestamp format: {date_cell:?}, {time_cell:?}"
        )),
    }
}

/// Extract image filename from Excel row data
pub fn extract_image_filename(row: &[Data], structure: &ExcelStructure) -> Option<String> {
    structure
        .image_col
        .and_then(|col| row.get(col))
        .and_then(|cell| match cell {
            Data::String(s) => Some(s.clone()),
            _ => None,
        })
}

/// Load Excel data from bytes
pub fn load_excel(file_data: Vec<u8>) -> Result<Vec<Vec<Data>>> {
    use calamine::{Reader, Xlsx, open_workbook_from_rs};
    use std::io::Cursor;

    let cursor = Cursor::new(file_data);
    let mut workbook: Xlsx<_> = open_workbook_from_rs(cursor)?;
    let sheet_name = workbook
        .sheet_names()
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No worksheets"))?;
    let worksheet = workbook.worksheet_range(&sheet_name)?;
    Ok(worksheet.rows().map(<[Data]>::to_vec).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKBOOK: &[u8] = include_bytes!("../../experiments/test_resources/merged.xlsx");
    const EXPECTED_TEXT_CELLS: usize = 7427;
    const EXPECTED_NUMERIC_SUM: f64 = 621_560_022.552_938_5;

    // A uniform cell-by-cell digest of a sheet: text, numbers and dates as the processor reads them.
    fn sheet_digest(rows: &[Vec<Data>]) -> (usize, usize, f64, usize) {
        let (mut sum, mut texts) = (0.0, 0);
        for cell in rows.iter().flatten() {
            match cell {
                Data::Float(f) => sum += f,
                Data::Int(i) => sum += *i as f64,
                Data::DateTime(d) => sum += d.as_f64(),
                Data::String(_) => texts += 1,
                _ => {}
            }
        }
        (rows.len(), rows[0].len(), sum, texts)
    }

    #[test]
    fn load_excel_reads_the_instrument_export_cell_for_cell() {
        let rows = load_excel(WORKBOOK.to_vec()).unwrap();
        assert_eq!(rows[0][2], Data::String("RTDTempX8".into()));
        assert_eq!(rows[6][0], Data::String("Date".into()));
        assert_eq!(rows[6][2], Data::String("Temperature 1 (°C)".into()));
        assert_eq!(rows[7][2], Data::Float(29.827));
        assert_eq!(rows[7][10], Data::String("INP_49640_2025-03-20_15-14-17".into()));
        match &rows[7][0] {
            Data::DateTime(d) => assert!((d.as_f64() - 45_736.634_571_759_26).abs() < 1e-9),
            other => panic!("first timestamp read as {other:?}"),
        }
        let last = rows.last().unwrap();
        assert_eq!(last[2], Data::Float(-18.988));
        assert_eq!(last[10], Data::String("INP_56492_2025-03-20_17-08-16".into()));
        let (n_rows, n_cols, sum, texts) = sheet_digest(&rows);
        assert_eq!((n_rows, n_cols), (6793, 205));
        assert_eq!(texts, EXPECTED_TEXT_CELLS);
        assert!((sum - EXPECTED_NUMERIC_SUM).abs() < 1e-3, "numeric sum {sum}");
    }

    #[test]
    fn load_excel_refuses_a_truncated_workbook_without_panicking() {
        let truncated = WORKBOOK[..WORKBOOK.len() / 2].to_vec();
        assert!(load_excel(truncated).is_err());
        assert!(load_excel(b"PK\x03\x04 not a zip".to_vec()).is_err());
        assert!(load_excel(Vec::new()).is_err());
    }

    #[test]
    fn test_extract_decimal() {
        assert_eq!(
            extract_decimal(&Data::Float(1.5)),
            Some(Decimal::try_from(1.5).unwrap())
        );
        assert_eq!(extract_decimal(&Data::Int(42)), Some(Decimal::from(42)));
        assert_eq!(extract_decimal(&Data::String("test".to_string())), None);
    }

    #[test]
    fn test_extract_integer() {
        assert_eq!(extract_integer(&Data::Int(42)), Some(42));
        assert_eq!(extract_integer(&Data::Float(42.7)), Some(43)); // Rounded
        assert_eq!(extract_integer(&Data::Float(42.3)), Some(42)); // Rounded
        assert_eq!(extract_integer(&Data::String("test".to_string())), None);
    }

    #[test]
    fn test_extract_image_filename() {
        let structure = ExcelStructure {
            date_col: 0,
            time_col: 1,
            image_col: Some(2),
            well_columns: std::collections::HashMap::new(),
            probe_columns: Vec::new(),
            data_start_row: 7,
        };

        let row = vec![
            Data::String("date".to_string()),
            Data::String("time".to_string()),
            Data::String("image.jpg".to_string()),
        ];

        assert_eq!(
            extract_image_filename(&row, &structure),
            Some("image.jpg".to_string())
        );

        let structure_no_image = ExcelStructure {
            image_col: None,
            ..structure
        };

        assert_eq!(extract_image_filename(&row, &structure_no_image), None);
    }
}
