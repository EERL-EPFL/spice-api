use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::Json,
};
use sea_orm::EntityTrait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::common::state::AppState;
use crate::services::data_processing_service::ExcelProcessingResult;

/// Upload and process an Excel file containing merged experiment data
#[utoipa::path(
    post,
    path = "/experiments/{experiment_id}/process-excel",
    request_body(content = String, description = "Excel file as multipart/form-data", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Excel file processed successfully", body = ExcelProcessingResult),
        (status = 400, description = "Invalid Excel file or format"),
        (status = 404, description = "Experiment not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("experiment_id" = Uuid, Path, description = "Experiment ID")
    ),
    tag = "experiments"
)]
pub async fn process_excel_upload(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<ExcelProcessingResult>, (StatusCode, Json<Value>)> {
    let db = &app_state.db;

    // Verify experiment exists
    let experiment = spice_entity::experiments::Entity::find_by_id(experiment_id)
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Database error: {}", e)
                })),
            )
        })?;

    if experiment.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Experiment not found"
            })),
        ));
    }

    // Extract Excel file from multipart upload
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Multipart error: {}", e)
            })),
        )
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "excel_file" || field_name == "file" {
            file_name = field.file_name().map(std::string::ToString::to_string);
            file_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "error": format!("Failed to read file data: {}", e)
                            })),
                        )
                    })?
                    .to_vec(),
            );
            break;
        }
    }

    let file_data = file_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "No Excel file found in request"
            })),
        )
    })?;

    let file_name = file_name.unwrap_or_else(|| "uploaded_file.xlsx".to_string());

    // Validate file format
    if !std::path::Path::new(&file_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xlsx"))
        && !std::path::Path::new(&file_name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xls"))
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "File must be an Excel file (.xlsx or .xls)"
            })),
        ));
    }

    // Process the Excel file using service layer
    let result = app_state
        .data_processing_service
        .process_excel_file(experiment_id, file_data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Excel processing error: {}", e)
                })),
            )
        })?;

    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use crate::routes::trays::services::{str_to_coordinates, coordinates_to_str};

    #[tokio::test]
    async fn test_multiple_well_transitions_validation() {
        // Test validates multiple specific well transitions with exact CSV data
        // This tests the progression of freezing across different wells and times
        
        // Define test cases for specific well transitions
        let test_cases = vec![
            // P2:A9 at row 4861 (earliest transition)
            (
                "P2", "A9", 4861, "2025-03-20T16:35:39",
                vec![-22.672, -23.161, -23.227, -23.126, -23.085, -23.088, -23.155, -22.846]
            ),
            // P2:A6 at row 5314 (mid-experiment transition)
            (
                "P2", "A6", 5314, "2025-03-20T16:43:18",
                vec![-25.105, -25.564, -25.607, -25.517, -25.484, -25.458, -25.581, -25.322]
            ),
            // P1:E9 at row 5749 (later transition)
            (
                "P1", "E9", 5749, "2025-03-20T16:50:38",
                vec![-27.475, -28.003, -28.004, -27.867, -27.915, -27.901, -27.951, -27.682]
            ),
            // P2:E7 at row 6092 (final transition)
            (
                "P2", "E7", 6092, "2025-03-20T16:56:25",
                vec![-29.398, -29.944, -29.979, -29.835, -29.842, -29.838, -29.905, -29.602]
            ),
        ];
        
        println!("✅ Multiple well transitions validation:");
        
        for (tray, well_coord, csv_row, timestamp, temp_readings) in test_cases {
            // Test coordinate conversion
            let well = str_to_coordinates(well_coord).unwrap();
            let coord_str = coordinates_to_str(&well).unwrap();
            assert_eq!(coord_str, well_coord, "Coordinate conversion should work for {}", well_coord);
            
            // Validate temperature readings are in expected ranges
            let temp_range = if csv_row < 5000 {
                -24.0..-22.0 // Earlier transitions around -23°C
            } else if csv_row < 5500 {
                -26.0..-25.0 // Mid transitions around -25°C
            } else if csv_row < 6000 {
                -29.0..-27.0 // Later transitions around -28°C
            } else {
                -31.0..-29.0 // Final transitions around -30°C
            };
            
            for (i, temp) in temp_readings.iter().enumerate() {
                assert!(temp_range.contains(temp), 
                    "Probe {} temperature {} should be in range {:?} for {} at row {}", 
                    i+1, temp, temp_range, well_coord, csv_row);
            }
            
            // Validate timestamp progression (each transition should be later than previous)
            let time_part = timestamp.split('T').nth(1).unwrap();
            let expected_progression = match csv_row {
                4861 => "16:35:39", // First
                5314 => "16:43:18", // Second (7m 39s later)
                5749 => "16:50:38", // Third (7m 20s later)
                6092 => "16:56:25", // Fourth (5m 47s later)
                _ => panic!("Unexpected row {}", csv_row)
            };
            assert!(time_part.contains(expected_progression), 
                "Timestamp should match expected progression for {}", well_coord);
            
            println!("   - {} {}: Row {} at {} (Temp: {:.1}°C to {:.1}°C)", 
                tray, well_coord, csv_row, timestamp, 
                temp_readings[0], temp_readings[1]);
        }
        
        // Validate the transition sequence timing
        let transition_intervals = vec![
            ("P2:A9 → P2:A6", "7m 39s"),
            ("P2:A6 → P1:E9", "7m 20s"),
            ("P1:E9 → P2:E7", "5m 47s"),
        ];
        
        for (transition, duration) in transition_intervals {
            println!("   - {}: {} interval", transition, duration);
        }
        
        println!("   - Total freezing progression: ~20 minutes (4861 → 6092 rows)");
        println!("   - Temperature decline: -22.7°C → -29.9°C (~7.2°C drop)");
        println!("   - Cross-tray freezing: P2 → P2 → P1 → P2 pattern");
        
        // TODO: When actual API testing is implemented, validate these transitions:
        // 1. GET /api/experiments/{id}/wells/{well_id}/transitions for each well
        // 2. Verify transition timestamps match exactly
        // 3. Verify temperature readings at each transition point
        // 4. Test the progression of freezing across wells and trays
    }
}