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
    if coordinate.len() < 2 {
        return Err(String::from("Invalid coordinate format"));
    }

    let column_char = coordinate.chars().next().unwrap();
    let row_str = &coordinate[1..];

    if !column_char.is_ascii_uppercase() || row_str.is_empty() {
        return Err(String::from("Invalid coordinate format"));
    }

    let column: u8 = (column_char as u8) - b'A' + 1;
    let row: u8 = row_str
        .parse()
        .map_err(|_| String::from("Invalid row number"))?;

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
        Err("Invalid coordinate format".into())
    );
    assert_eq!(str_to_coordinates("A0"), Err("Invalid row number".into()));
}
