use sea_orm::entity::prelude::*;
use sea_orm::{DatabaseConnection, DbBackend, Statement};
use serde_json::{Value, json};

/// Generate a convex hull GeoJSON for all samples belonging to a location
pub async fn get_location_convex_hull(db: &DatabaseConnection, location_id: Uuid) -> Option<Value> {
    // Only works with PostgreSQL/PostGIS
    if db.get_database_backend() != DbBackend::Postgres {
        return None;
    }

    let raw_sql = r"
    SELECT location.id,
           ST_AsGeoJSON(
               ST_Buffer(
                   ST_ConvexHull(ST_Collect(samples.geom)),
                   0.001  -- ~100m buffer in degrees
               )
           ) AS convex_hull
    FROM locations location
    JOIN samples ON samples.location_id = location.id
    WHERE location.id = $1
      AND samples.geom IS NOT NULL
    GROUP BY location.id
    HAVING COUNT(samples.geom) > 0
    ";

    // Execute the query
    if let Ok(result) = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            raw_sql,
            vec![location_id.into()],
        ))
        .await
    {
        let Some(row) = result else {
            return Some(json!({"type": "FeatureCollection", "features": []}));
        };
        
        if let Ok(convex_hull) = row.try_get::<String>("", "convex_hull") {
            if let Ok(parsed_geojson) = serde_json::from_str(&convex_hull) {
                return Some(parsed_geojson);
            }
        }
    }

    // Return empty FeatureCollection if query fails or no valid geometry
    Some(json!({"type": "FeatureCollection", "features": []}))
}

/// Generate a convex hull GeoJSON for samples with a buffer for visibility
pub async fn get_location_convex_hull_buffered(
    db: &DatabaseConnection, 
    location_id: Uuid, 
    buffer_meters: f64
) -> Option<Value> {
    // Only works with PostgreSQL/PostGIS
    if db.get_database_backend() != DbBackend::Postgres {
        return None;
    }

    let raw_sql = r"
    SELECT location.id,
           ST_AsGeoJSON(
               ST_Transform(
                   ST_Buffer(
                       ST_Transform(
                           ST_ConvexHull(ST_Collect(samples.geom)), 
                           3857  -- Web Mercator for meter-based buffer
                       ), 
                       $2
                   ),
                   4326  -- Convert back to WGS84
               )
           ) AS convex_hull
    FROM locations location
    JOIN samples ON samples.location_id = location.id
    WHERE location.id = $1
      AND samples.geom IS NOT NULL
    GROUP BY location.id
    HAVING COUNT(samples.geom) > 0
    ";

    // Execute the query
    if let Ok(result) = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            raw_sql,
            vec![location_id.into(), buffer_meters.into()],
        ))
        .await
    {
        let Some(row) = result else {
            return Some(json!({"type": "FeatureCollection", "features": []}));
        };
        
        if let Ok(convex_hull) = row.try_get::<String>("", "convex_hull") {
            if let Ok(parsed_geojson) = serde_json::from_str(&convex_hull) {
                return Some(parsed_geojson);
            }
        }
    }

    // Return empty FeatureCollection if query fails or no valid geometry
    Some(json!({"type": "FeatureCollection", "features": []}))
}

