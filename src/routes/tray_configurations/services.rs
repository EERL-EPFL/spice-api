#[derive(Debug, PartialEq)]
pub struct WellCoordinate {
    pub column: u8,
    pub row: u8,
}

pub fn str_to_coordinates(coordinate: &str) -> Result<WellCoordinate, String> {
    if coordinate.len() < 2
        || !coordinate.chars().next().unwrap().is_ascii_uppercase()
        || !coordinate.chars().nth(1).unwrap().is_ascii_digit()
    {
        return Err(format!(
            "Invalid coordinate format, must be like 'A1', provided: {coordinate}"
        ));
    }

    let mut chars = coordinate.chars();
    let column_char = chars.next().unwrap();
    let row_str: String = chars.collect();

    if !column_char.is_ascii_uppercase()
        || row_str.is_empty()
        || row_str.len() != coordinate.len() - 1
    {
        return Err("Invalid coordinate format".into());
    }

    let row: u8 = (column_char as u8) - b'A' + 1; // A=1, B=2 (row number)
    let column: u8 = row_str
        .parse()
        .map_err(|_| format!("Invalid column number: {coordinate}"))?;

    if column < 1 {
        return Err("Invalid column number, must be a positive integer".into());
    }
    Ok(WellCoordinate { column, row })
}

#[test]
fn test_str_to_coordinates() {
    assert_eq!(
        str_to_coordinates("A1"),
        Ok(WellCoordinate { column: 1, row: 1 }) // A=row 1, 1=col 1
    );
    assert_eq!(
        str_to_coordinates("Z1"),
        Ok(WellCoordinate { column: 1, row: 26 }) // Z=row 26, 1=col 1
    );
    assert_eq!(
        str_to_coordinates("AA1"),
        Err("Invalid coordinate format, must be like 'A1', provided: AA1".into())
    );
    assert_eq!(
        str_to_coordinates("A0"),
        Err("Invalid column number, must be a positive integer".into())
    );
    assert_eq!(
        str_to_coordinates("H12"),
        Ok(WellCoordinate { column: 12, row: 8 }) // H=row 8, 12=col 12
    );
}
