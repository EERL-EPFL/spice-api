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
    if coord.row > 26 {
        return Err("Only supports A-Z for rows".into());
    }

    Ok(format!(
        "{}{}",
        char::from(b'A' + (coord.row - 1)),       // row determines letter (A, B, C...)
        coord.column                              // column determines number (1, 2, 3...)
    ))
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

    let row: u8 = (column_char as u8) - b'A' + 1;      // A=1, B=2 (row number)
    let column: u8 = row_str
        .parse()
        .map_err(|_| format!("Invalid column number: {coordinate}"))?;

    if column < 1 {
        return Err("Invalid column number, must be a positive integer".into());
    }
    Ok(WellCoordinate { column, row })
}

// Transform coordinates based on tray rotation to match UI display logic
// This matches the getDisplayIndices function in TrayDisplay.tsx
pub fn transform_coordinates_for_rotation(
    coord: &WellCoordinate,
    rotation_degrees: i32,
    qty_cols: u8,
    qty_rows: u8,
) -> Result<WellCoordinate, String> {
    // Convert 1-based coordinates to 0-based for calculations
    let logical_row = (coord.row - 1) as i32;
    let logical_col = (coord.column - 1) as i32;
    let qty_x = qty_cols as i32;  // number of columns
    let qty_y = qty_rows as i32;  // number of rows
    
    // Apply the same transformation as TrayDisplay.tsx getDisplayIndices
    let (x_index, y_index) = match rotation_degrees {
        90 => (logical_row, qty_x - 1 - logical_col),
        180 => (qty_x - 1 - logical_col, qty_y - 1 - logical_row), 
        270 => (qty_y - 1 - logical_row, logical_col),
        _ => (logical_col, logical_row), // 0 degrees or invalid
    };
    
    // Convert back to 1-based coordinates
    // For coordinate string generation: column (x_index) becomes the number, row (y_index) becomes the letter
    Ok(WellCoordinate {
        column: (x_index + 1) as u8,
        row: (y_index + 1) as u8,
    })
}

#[test]
fn test_coordinates_to_str() {
    assert_eq!(
        coordinates_to_str(&WellCoordinate { column: 1, row: 1 }),  // A1
        Ok("A1".into())
    );
    assert_eq!(
        coordinates_to_str(&WellCoordinate { column: 1, row: 26 }), // Z1
        Ok("Z1".into())
    );
    assert_eq!(
        coordinates_to_str(&WellCoordinate { column: 0, row: 1 }),
        Err("Invalid coordinate".into())
    );
    assert_eq!(
        coordinates_to_str(&WellCoordinate { column: 1, row: 27 }),
        Err("Only supports A-Z for rows".into())
    );
}

#[test]
fn test_str_to_coordinates() {
    assert_eq!(
        str_to_coordinates("A1"),
        Ok(WellCoordinate { column: 1, row: 1 })    // A=row 1, 1=col 1
    );
    assert_eq!(
        str_to_coordinates("Z1"),
        Ok(WellCoordinate { column: 1, row: 26 })   // Z=row 26, 1=col 1
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
        Ok(WellCoordinate { column: 12, row: 8 })   // H=row 8, 12=col 12
    );
}

#[test]
fn test_transform_coordinates_for_rotation() {
    // Test 12x8 tray (12 columns, 8 rows A-H)
    let qty_cols = 12; // columns  
    let qty_rows = 8;  // rows
    
    // Test 0 degrees (no rotation)
    assert_eq!(
        transform_coordinates_for_rotation(&WellCoordinate { column: 1, row: 1 }, 0, qty_cols, qty_rows),
        Ok(WellCoordinate { column: 1, row: 1 })
    );
    
    // Test 90 degrees rotation - matches TrayDisplay.tsx logic
    // A1 (logical row=0, col=0) -> xIndex=0, yIndex=11 -> column=1, row=12
    assert_eq!(
        transform_coordinates_for_rotation(&WellCoordinate { column: 1, row: 1 }, 90, qty_cols, qty_rows),
        Ok(WellCoordinate { column: 1, row: 12 })
    );
    
    // Test 180 degrees rotation
    // A1 (logical row=0, col=0) -> xIndex=11, yIndex=7 -> column=12, row=8 (H12)
    assert_eq!(
        transform_coordinates_for_rotation(&WellCoordinate { column: 1, row: 1 }, 180, qty_cols, qty_rows),
        Ok(WellCoordinate { column: 12, row: 8 })
    );
    
    // Test 270 degrees rotation
    // A1 (logical row=0, col=0) -> xIndex=7, yIndex=0 -> column=8, row=1 (A8)  
    assert_eq!(
        transform_coordinates_for_rotation(&WellCoordinate { column: 1, row: 1 }, 270, qty_cols, qty_rows),
        Ok(WellCoordinate { column: 8, row: 1 })
    );

    // Debug the exact issue: what logical coordinate at 270° should show as E8?
    // E8 means: row E (5th row), column 8
    // In coordinates_to_str: letter = row-1, number = column
    // So E8 = WellCoordinate { column: 8, row: 5 }
    
    // Working backwards from 270° transformation:
    // Case 270: xIndex = qty_y - 1 - logical_row, yIndex = logical_col
    // We want: xIndex + 1 = 8 (column), yIndex + 1 = 5 (row)
    // So: xIndex = 7, yIndex = 4
    // Therefore: qty_y - 1 - logical_row = 7 -> logical_row = 8 - 1 - 7 = 0
    // And: logical_col = 4
    // So logical A5 should become E8 at 270°
    println!("Debug: A5 at 270° should become E8:");
    let test_a5 = transform_coordinates_for_rotation(&WellCoordinate { column: 5, row: 1 }, 270, qty_cols, qty_rows);
    println!("  A5 -> {:?}", test_a5);
    
    // Let's also test the reverse - what shows the current wrong result D5?
    println!("Debug: What logical coordinate gives D5?");
    // D5 = WellCoordinate { column: 5, row: 4 }
    // At 270°: column=5 means xIndex=4, row=4 means yIndex=3
    // So: qty_y - 1 - logical_row = 4 -> logical_row = 3, logical_col = 3
    // So logical D4 should become D5 at 270°
    let test_d4 = transform_coordinates_for_rotation(&WellCoordinate { column: 4, row: 4 }, 270, qty_cols, qty_rows);
    println!("  D4 -> {:?}", test_d4);
}
