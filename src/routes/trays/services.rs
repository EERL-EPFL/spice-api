#[derive(Debug, PartialEq)]
pub struct WellCoordinate {
    pub column: u8,
    pub row: u8,
}

// Convert row/col to coordinate (A1, B2, etc.)
pub fn coordinates_to_str(coord: &WellCoordinate) -> Result<String, String> {
    if coord.column == 0 || coord.row == 0 {
        return Err(String::from("Invalid coordinate"));
    }
    if coord.column > 26 {
        return Err(String::from("Only supports A-Z for columns"));
    }

    Ok(format!(
        "{}{}",
        char::from(b'A' + (coord.column - 1)),
        coord.row
    ))
}

pub fn str_to_coordinates(coordinate: &str) -> Result<WellCoordinate, String> {
    if coordinate.len() < 2
        || !coordinate.chars().next().unwrap().is_ascii_uppercase()
        || !coordinate.chars().nth(1).unwrap().is_ascii_digit()
    {
        return Err(format!(
            "Invalid coordinate format, must be like 'A1', provided: {}",
            coordinate
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

    let column: u8 = (column_char as u8) - b'A' + 1;
    let row: u8 = row_str
        .parse()
        .map_err(|_| format!("Invalid row number: {coordinate}"))?;

    if row < 1 {
        return Err("Invalid row number, must be a positive integer".into());
    }
    Ok(WellCoordinate { column, row })
}

#[test]
fn test_coordinates_to_str() {
    assert_eq!(
        coordinates_to_str(&WellCoordinate { column: 1, row: 1 }),
        Ok("A1".into())
    );
    assert_eq!(
        coordinates_to_str(&WellCoordinate { column: 26, row: 1 }),
        Ok("Z1".into())
    );
    assert_eq!(
        coordinates_to_str(&WellCoordinate { column: 0, row: 1 }),
        Err("Invalid coordinate".into())
    );
    assert_eq!(
        coordinates_to_str(&WellCoordinate { column: 27, row: 1 }),
        Err("Only supports A-Z for columns".into())
    );
}

#[test]
fn test_str_to_coordinates() {
    assert_eq!(
        str_to_coordinates("A1"),
        Ok(WellCoordinate { column: 1, row: 1 })
    );
    assert_eq!(
        str_to_coordinates("Z1"),
        Ok(WellCoordinate { column: 26, row: 1 })
    );
    assert_eq!(
        str_to_coordinates("AA1"),
        Err("Invalid coordinate format, must be like 'A1', provided: AA1".into())
    );
    assert_eq!(
        str_to_coordinates("A0"),
        Err("Invalid row number, must be a positive integer".into())
    );
    assert_eq!(
        str_to_coordinates("H12"),
        Ok(WellCoordinate { column: 8, row: 12 })
    );
}
