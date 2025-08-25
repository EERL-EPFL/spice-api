#!/usr/bin/env cargo
//! SPICE Database Seeder
//!
//! A terminal application for seeding the SPICE database with realistic test data.
//! This tool creates projects, locations, samples, treatments, tray configurations,
//! experiments, and processes the actual merged.xlsx file from test resources.
//!
//! Usage:
//!   `cargo run --bin seed_database -- --url http://localhost:3000 --token YOUR_JWT_TOKEN`
//!
//! Features:
//! - Realistic scientific data based on ice nucleation research patterns
//! - Processes actual Excel test file (merged.xlsx)
//! - Beautiful terminal UI with progress indicators
//! - JWT authentication for secured endpoints
//! - Comprehensive error handling and logging

use chrono::{Duration as ChronoDuration, Utc};
use clap::{Arg, Command};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use rand::Rng;
use reqwest::{Client, multipart};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{Duration, sleep};
use futures::future::join_all;

#[derive(Debug, Clone)]
pub struct SeedingConfig {
    pub base_url: String,
    pub jwt_token: String,
    pub client: Client,
}

#[derive(Debug, Default)]
pub struct CreatedObjects {
    pub projects: Vec<Value>,
    pub locations: Vec<Value>,
    pub samples: Vec<Value>,
    pub treatments: Vec<Value>,
    pub tray_configurations: Vec<Value>,
    pub experiments: Vec<Value>,
    pub processed_experiments: Vec<Value>,
}

pub struct DatabaseSeeder {
    config: SeedingConfig,
    created_objects: CreatedObjects,
}

/// Generate a random coordinate within approximately 1km radius of a base coordinate
fn generate_nearby_coordinate(base_lat: f64, base_lon: f64) -> (f64, f64) {
    let mut rng = rand::rng();

    // Approximate degrees per km (varies with latitude, but good enough for testing)
    // 1 degree latitude ≈ 111 km, so 0.009 degrees ≈ 1 km
    let lat_per_km = 0.009;
    let lon_per_km = 0.009 / base_lat.to_radians().cos(); // Adjust for latitude

    // Generate random offset within 1km circle
    let angle = rng.random::<f64>() * 2.0 * std::f64::consts::PI;
    let radius = rng.random::<f64>().sqrt(); // Sqrt for uniform distribution in circle

    let lat_offset = lat_per_km * radius * angle.sin();
    let lon_offset = lon_per_km * radius * angle.cos();

    (base_lat + lat_offset, base_lon + lon_offset)
}

impl DatabaseSeeder {
    pub fn new(base_url: String, jwt_token: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        Self {
            config: SeedingConfig {
                base_url: base_url.trim_end_matches('/').to_string(),
                jwt_token,
                client,
            },
            created_objects: CreatedObjects::default(),
        }
    }

    /// Make multiple requests in parallel with controlled concurrency
    async fn make_parallel_requests(
        &self,
        requests: Vec<(String, String, Option<Value>)>, // (method, endpoint, data)
        max_concurrent: usize,
        pb: &ProgressBar,
    ) -> Result<Vec<Value>, String> {
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let mut tasks = Vec::new();

        for (method, endpoint, data) in requests {
            let sem = Arc::clone(&semaphore);
            let config = self.config.clone();
            let pb_clone = pb.clone();
            
            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                
                let client = &config.client;
                let url = format!("{}{}", config.base_url, endpoint);

                let response = match method.to_uppercase().as_str() {
                    "POST" => {
                        let mut request = client
                            .post(&url)
                            .header("authorization", format!("Bearer {}", config.jwt_token))
                            .header("content-type", "application/json");
                        if let Some(json_data) = data {
                            request = request.json(&json_data);
                        }
                        request.send().await
                    },
                    "GET" => {
                        client
                            .get(&url)
                            .header("authorization", format!("Bearer {}", config.jwt_token))
                            .send()
                            .await
                    },
                    _ => return Err("Unsupported HTTP method".to_string()),
                };

                let result = match response {
                    Ok(resp) if resp.status().is_success() => {
                        resp.json::<Value>().await
                            .map_err(|e| format!("JSON parse error: {e}"))
                    },
                    Ok(resp) => {
                        let status = resp.status();
                        let error_text = resp.text().await.unwrap_or_default();
                        Err(format!("HTTP {} {}: {}", status, endpoint, error_text))
                    },
                    Err(e) => Err(format!("Request error {}: {e}", endpoint)),
                };

                pb_clone.inc(1);
                result
            });
            
            tasks.push(task);
        }

        let results: Result<Vec<_>, String> = join_all(tasks).await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Task join error: {}", e))?
            .into_iter()
            .collect();

        results
    }

    async fn make_request(
        &self,
        method: &str,
        endpoint: &str,
        data: Option<Value>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}{}", self.config.base_url, endpoint);

        let response = match method.to_uppercase().as_str() {
            "GET" => {
                self.config
                    .client
                    .get(&url)
                    .header("authorization", format!("Bearer {}", self.config.jwt_token))
                    .send()
                    .await?
            }
            "POST" => {
                let mut request = self
                    .config
                    .client
                    .post(&url)
                    .header("authorization", format!("Bearer {}", self.config.jwt_token))
                    .header("content-type", "application/json");
                if let Some(json_data) = data {
                    request = request.json(&json_data);
                }
                request.send().await?
            }
            "PATCH" => {
                let mut request = self
                    .config
                    .client
                    .patch(&url)
                    .header("authorization", format!("Bearer {}", self.config.jwt_token))
                    .header("content-type", "application/json");
                if let Some(json_data) = data {
                    request = request.json(&json_data);
                }
                request.send().await?
            }
            _ => return Err("Unsupported HTTP method".into()),
        };

        if response.status().is_success() {
            let result = response.json::<Value>().await?;
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(format!("HTTP {} {}: {}", status, endpoint, error_text).into())
        }
    }

    async fn make_multipart_request(
        &self,
        endpoint: &str,
        file_path: &PathBuf,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}{}", self.config.base_url, endpoint);

        let file_content = fs::read(file_path)?;
        let file_name = file_path.file_name().unwrap().to_str().unwrap();

        let form = multipart::Form::new().part(
            "file",
            multipart::Part::bytes(file_content)
                .file_name(file_name.to_string())
                .mime_str("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")?,
        );

        let response = self
            .config
            .client
            .post(&url)
            .header("authorization", format!("Bearer {}", self.config.jwt_token))
            .multipart(form)
            .send()
            .await?;

        if response.status().is_success() {
            let result = response.json::<Value>().await?;
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(format!("HTTP {} {}: {}", status, endpoint, error_text).into())
        }
    }

    pub async fn test_connection(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Health check removed for load balancer compatibility
        Ok(())
    }

    pub async fn create_projects(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "{} Creating research projects...",
            style("[1/7]").bold().dim()
        );

        let projects_data = vec![
            json!({
                "name": "EERL Arctic INP Study 2024",
                "colour": "#2563EB",
                "note": "Comprehensive analysis of ice nucleation particles in Arctic atmospheric samples - Environmental and Energy Research Laboratory"
            }),
            json!({
                "name": "Agricultural Frost Protection Research",
                "colour": "#16A34A",
                "note": "Investigation of biological ice nucleation for agricultural frost protection strategies in temperate climates"
            }),
            json!({
                "name": "Marine Boundary Layer INP Analysis",
                "colour": "#7C3AED",
                "note": "Characterization of ice-active particles in marine aerosols and their impact on cloud formation"
            }),
            json!({
                "name": "Climate Model Validation Project",
                "colour": "#DC2626",
                "note": "Experimental validation of ice nucleation parameterizations for improved climate model accuracy"
            }),
        ];

        let pb = ProgressBar::new(projects_data.len() as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-"));

        for project_data in projects_data {
            let name = project_data["name"].as_str().unwrap();
            pb.set_message(format!("Creating: {}", name));

            let result = self
                .make_request("POST", "/projects", Some(project_data))
                .await?;
            self.created_objects.projects.push(result);

            pb.inc(1);
            sleep(Duration::from_millis(100)).await; // Small delay for better UX
        }

        pb.finish_with_message("Projects created!");
        println!(
            "{} Created {} projects",
            style("✅").green(),
            self.created_objects.projects.len()
        );

        Ok(())
    }

    pub async fn create_locations(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "{} Creating sampling locations...",
            style("[2/7]").bold().dim()
        );

        let location_sets = vec![
            // Arctic research sites
            vec![
                (
                    "Utqiagvik Research Station",
                    "Arctic Ocean coastal site, formerly Barrow - continuous atmospheric monitoring",
                    71.323020,
                    -156.766045, // Utqiagvik, Alaska
                ),
                (
                    "Greenland Summit Station",
                    "High-altitude ice sheet site at 3200m elevation - pristine atmospheric conditions",
                    72.579630,
                    -38.459210, // Summit Station, Greenland
                ),
                (
                    "Svalbard Zeppelin Observatory",
                    "Arctic atmospheric research station - long-term aerosol monitoring",
                    78.906700,
                    11.888315, // Ny-Ålesund, Svalbard
                ),
            ],
            // Agricultural research sites
            vec![
                (
                    "Iowa State Agricultural Research",
                    "Midwest corn belt sampling site - agricultural aerosol characterization",
                    42.030800,
                    -93.631925, // Ames, Iowa
                ),
                (
                    "California Central Valley Station",
                    "Prime agricultural region - dust and biological aerosol source",
                    36.778315,
                    -119.417865, // Fresno, California
                ),
                (
                    "Wisconsin Dairy Research Facility",
                    "Rural agricultural site - biological particle emissions",
                    43.064200,
                    -89.401230, // Madison, Wisconsin
                ),
            ],
            // Marine boundary layer sites
            vec![
                (
                    "Point Reyes Marine Laboratory",
                    "California coastal site - marine boundary layer sampling",
                    38.098415,
                    -122.819445, // Point Reyes, California
                ),
                (
                    "Bermuda Atlantic Time-series",
                    "Mid-Atlantic marine aerosol characterization site",
                    32.169025,
                    -64.834050, // Bermuda
                ),
                (
                    "Cape Cod Marine Station",
                    "Northwest Atlantic coastal marine aerosol research",
                    41.668835,
                    -70.296240, // Cape Cod, Massachusetts
                ),
            ],
            // Climate validation sites
            vec![
                (
                    "NOAA Mauna Loa Observatory",
                    "High-altitude atmospheric baseline measurements",
                    19.536250,
                    -155.576300, // Mauna Loa, Hawaii
                ),
                (
                    "ARM Southern Great Plains",
                    "Continental atmospheric research facility - comprehensive measurements",
                    36.605800,
                    -97.488750, // Lamont, Oklahoma
                ),
                (
                    "Jungfraujoch High Altitude Station",
                    "European high-altitude atmospheric research - 3580m elevation",
                    46.547735,
                    7.985025, // Jungfraujoch, Switzerland
                ),
            ],
        ];

        let total_locations: usize = location_sets.iter().map(|set| set.len()).sum();
        let pb = ProgressBar::new(total_locations as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-"));

        for (i, project) in self.created_objects.projects.iter().enumerate() {
            let project_id = project["id"].as_str().unwrap();
            let locations_for_project = &location_sets[i % location_sets.len()];

            for (name, comment, latitude, longitude) in locations_for_project {
                pb.set_message(format!("Creating: {}", name));

                let location_data = json!({
                    "name": name,
                    "comment": format!("{} (GPS: {:.6}°, {:.6}°)", comment, latitude, longitude),
                    "project_id": project_id
                });

                // Store coordinates for use in samples
                let _location_coords = (latitude, longitude);

                let result = self
                    .make_request("POST", "/locations", Some(location_data))
                    .await?;

                // Store location with coordinates for sample creation (6 decimal places)
                let mut location_with_coords = result.clone();
                location_with_coords["latitude"] = json!(format!("{:.6}", latitude));
                location_with_coords["longitude"] = json!(format!("{:.6}", longitude));

                self.created_objects.locations.push(location_with_coords);

                pb.inc(1);
                sleep(Duration::from_millis(50)).await;
            }
        }

        pb.finish_with_message("Locations created!");
        println!(
            "{} Created {} locations",
            style("✅").green(),
            self.created_objects.locations.len()
        );

        Ok(())
    }

    pub async fn create_samples(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "{} Creating environmental samples...",
            style("[3/7]").bold().dim()
        );

        let sample_patterns = vec![
            ("PM10 Aerosol Filter", "filter"),
            ("Atmospheric Bulk Sample", "bulk"),
            ("Size-Segregated Aerosol", "filter"),
            ("Fog Water Collection", "bulk"),
            ("Snow Sample", "bulk"),
            ("Ice Core Sample", "bulk"),
            ("Aerosol Impactor Sample", "filter"),
            ("TSP Collection", "filter"),
            ("Precipitation Sample", "bulk"),
            ("Surface Water Sample", "bulk"),
            ("Demineralised Water Blank", "procedural_blank"),
            ("Filter Blank Control", "procedural_blank"),
            ("Collection System Blank", "procedural_blank"),
            ("Field Blank Control", "procedural_blank"),
        ];

        let samples_per_location = 100; // Generate 100 samples per location for geographic spread
        let total_samples = self.created_objects.locations.len() * samples_per_location;
        
        println!("DEBUG: Planning to create {} total samples ({} locations × {} samples/location)", 
                 total_samples, self.created_objects.locations.len(), samples_per_location);

        let pb = ProgressBar::new(total_samples as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-"));

        // Process each location sequentially but with batch creation for reliability
        for location in &self.created_objects.locations {
            let location_id = location["id"].as_str().unwrap();
            let location_name = location["name"].as_str().unwrap();
            let base_latitude = location["latitude"]
                .as_str()
                .unwrap_or("0.0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let base_longitude = location["longitude"]
                .as_str()
                .unwrap_or("0.0")
                .parse::<f64>()
                .unwrap_or(0.0);

            pb.set_message(format!("Creating samples for {}", &location_name[..location_name.len().min(20)]));

            // Create samples for this location in smaller batches
            let batch_size = 10;
            for batch_start in (0..samples_per_location).step_by(batch_size) {
                let batch_end = (batch_start + batch_size).min(samples_per_location);
                let mut batch_requests = Vec::new();

                for i in batch_start..batch_end {
                    let (pattern_name, sample_type) = &sample_patterns[i % sample_patterns.len()];

                    // Generate nearby coordinates within ~1km radius
                    let (sample_lat, sample_lon) =
                        generate_nearby_coordinate(base_latitude, base_longitude);

                    // Vary collection dates over several months
                    let days_offset = ((i as i64 * 3) % 180) + 1; // 1-180 days ago
                    let collection_date = (Utc::now() - ChronoDuration::days(days_offset))
                        .format("%Y-%m-%d")
                        .to_string();

                    // Add sequence number for uniqueness
                    let sample_name = format!(
                        "{} {} {} S{:03}",
                        &location_name[..location_name.len().min(15)],
                        pattern_name,
                        collection_date,
                        i + 1
                    );

                    let remarks = match *sample_type {
                        "procedural_blank" => format!(
                            "Quality control {} (S{:03}) processed alongside environmental samples on {}. GPS: {:.6}°, {:.6}°",
                            pattern_name.to_lowercase(),
                            i + 1,
                            collection_date,
                            sample_lat,
                            sample_lon
                        ),
                        _ => format!(
                            "Environmental sample S{:03} collected from {} region on {}. Sample type: {}. GPS: {:.6}°, {:.6}°",
                            i + 1,
                            location_name,
                            collection_date,
                            sample_type,
                            sample_lat,
                            sample_lon
                        ),
                    };

                    // Generate realistic field values based on sample type and INSEKT document
                    let mut sample_data = json!({
                        "name": sample_name,
                        "type": sample_type,
                        "remarks": remarks,
                        "well_volume_litres": 0.00005 // 50μL default per INSEKT document
                    });

                    // Type-specific fields based on INSEKT document specifications
                    match *sample_type {
                        "procedural_blank" => {
                            // Procedural blanks have no location, dates, or volumes per INSEKT
                            sample_data["location_id"] = json!(null);
                        }
                        "filter" => {
                            // Filter sample fields from INSEKT document
                            let mut rng = rand::rng();
                            sample_data["location_id"] = json!(location_id);
                            sample_data["start_time"] = json!(format!("{}T{:02}:{:02}:00Z", 
                                collection_date, 
                                6 + (i % 12), // Sampling hours: 6-18
                                (i * 7) % 60  // Minutes variation
                            ));
                            sample_data["stop_time"] = json!(format!("{}T{:02}:{:02}:00Z", 
                                collection_date, 
                                8 + (i % 12), // 2 hour sampling duration
                                (i * 7) % 60
                            ));
                            sample_data["flow_litres_per_minute"] = json!(rng.gen_range(8.0..15.0)); // Typical aerosol sampling rates
                            sample_data["total_volume"] = json!(rng.gen_range(500.0..2000.0)); // Total air volume sampled
                            sample_data["suspension_volume_litres"] = json!(rng.gen_range(0.008..0.020)); // 8-20mL suspension
                            sample_data["filter_substrate"] = json!(match i % 3 {
                                0 => "PTFE",
                                1 => "Polycarbonate", 
                                _ => "Quartz fiber"
                            });
                        }
                        "bulk" => {
                            // Bulk sample fields from INSEKT document  
                            let mut rng = rand::rng();
                            sample_data["location_id"] = json!(location_id);
                            sample_data["latitude"] = json!(format!("{:.6}", sample_lat));
                            sample_data["longitude"] = json!(format!("{:.6}", sample_lon));
                            sample_data["start_time"] = json!(format!("{}T{:02}:{:02}:00Z", 
                                collection_date,
                                8 + (i % 8), // Collection hours: 8-16
                                (i * 11) % 60
                            ));
                            sample_data["suspension_volume_litres"] = json!(rng.gen_range(0.010..0.050)); // 10-50mL suspension
                            sample_data["air_volume_litres"] = json!(rng.gen_range(0.001..0.005)); // Air volume displaced
                            sample_data["water_volume_litres"] = json!(rng.gen_range(0.008..0.045)); // Water for suspension  
                            sample_data["initial_concentration_gram_l"] = json!(rng.gen_range(0.1..2.0)); // Initial concentration
                        }
                        _ => {
                            sample_data["location_id"] = json!(location_id);
                        }
                    }

                    batch_requests.push(("POST".to_string(), "/samples".to_string(), Some(sample_data)));
                }

                // Execute this batch with limited concurrency
                let batch_results = self.make_parallel_requests(batch_requests, 5, &pb).await
                    .map_err(|e| format!("Batch sample creation failed: {}", e))?;
                
                self.created_objects.samples.extend(batch_results);
                
                // Small delay between batches to avoid overwhelming the server
                sleep(Duration::from_millis(100)).await;
            }
        }

        println!("DEBUG: Final sample count: {}", self.created_objects.samples.len());

        pb.finish_with_message("Samples created!");
        println!(
            "{} Created {} samples",
            style("✅").green(),
            self.created_objects.samples.len()
        );

        Ok(())
    }

    pub async fn create_treatments(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "{} Creating laboratory treatments...",
            style("[4/7]").bold().dim()
        );

        let treatment_descriptions = HashMap::from([
            (
                "none",
                "Untreated control sample - baseline ice nucleation activity",
            ),
            (
                "heat",
                "Heat treatment at 95°C for 20 minutes - removes heat-labile biological INPs",
            ),
            (
                "h2o2",
                "Hydrogen peroxide treatment - removes organic components including biological INPs",
            ),
        ]);

        let mut total_treatments = 0;
        for sample in &self.created_objects.samples {
            let sample_type = sample
                .get("sample_type")
                .and_then(|v| v.as_str())
                .unwrap_or("bulk");
            let treatment_count = match sample_type {
                "procedural_blank" | "pure_water" => 1, // only "none"
                "filter" | "bulk" => 3,                 // none, heat, h2o2
                _ => 2,                                  // none, heat
            };
            total_treatments += treatment_count;
        }

        let pb = ProgressBar::new(total_treatments as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-"));

        for sample in &self.created_objects.samples {
            let sample_id = sample["id"].as_str().unwrap();
            let sample_name = sample["name"].as_str().unwrap();
            let sample_type = sample
                .get("sample_type")
                .and_then(|v| v.as_str())
                .unwrap_or("bulk");

            // Determine treatments based on sample type
            let treatments = match sample_type {
                "procedural_blank" => vec!["none"],
                "pure_water" => vec!["none"],
                "filter" | "bulk" => vec!["none", "heat", "h2o2"],
                _ => vec!["none", "heat"],
            };

            for treatment_name in treatments {
                pb.set_message(format!(
                    "{}... - {}",
                    &sample_name[..sample_name.len().min(25)],
                    treatment_name
                ));

                // Add enzyme volume for treatments based on type
                let mut treatment_data = json!({
                    "sample_id": sample_id,
                    "name": treatment_name,
                    "notes": treatment_descriptions[treatment_name]
                });

                // Add realistic enzyme volumes for certain treatments
                if treatment_name == "h2o2" {
                    let mut rng = rand::rng();
                    treatment_data["enzyme_volume_litres"] = json!(rng.gen_range(0.0001..0.0005)); // 0.1-0.5mL
                }

                let result = self
                    .make_request("POST", "/treatments", Some(treatment_data))
                    .await?;
                self.created_objects.treatments.push(result);

                pb.inc(1);
                sleep(Duration::from_millis(20)).await;
            }
        }

        pb.finish_with_message("Treatments created!");
        println!(
            "{} Created {} treatments",
            style("✅").green(),
            self.created_objects.treatments.len()
        );

        Ok(())
    }

    pub async fn create_tray_configurations(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "{} Creating tray configurations...",
            style("[5/7]").bold().dim()
        );

        let tray_configs = vec![json!({
            "name": "INP freezing assay",
            "experiment_default": true,
            "trays": [
                {
                    "name": "P1",
                    "rotation_degrees": 90,
                    "well_relative_diameter": 6.4,
                    "qty_cols": 12,
                    "qty_rows": 8,
                    "probe_locations": [
                        {
                            "data_column_index": 1,
                            "position_x": 22.1,
                            "position_y": 77.6,
                            "name": "Probe 1"
                        },
                        {
                            "data_column_index": 2,
                            "position_x": 47.1,
                            "position_y": 20,
                            "name": "Probe 2"
                        },
                        {
                            "data_column_index": 3,
                            "position_x": 113,
                            "position_y": 19.5,
                            "name": "Probe 3"
                        },
                        {
                            "data_column_index": 4,
                            "position_x": 143.5,
                            "position_y": 79.5,
                            "name": "Probe 4"
                        }
                    ],
                    "upper_left_corner_x": 416,
                    "upper_left_corner_y": 75,
                    "lower_right_corner_x": 135,
                    "lower_right_corner_y": 542,
                    "order_sequence": 1
                },
                {
                    "name": "P2",
                    "rotation_degrees": 270,
                    "well_relative_diameter": 6.4,
                    "qty_cols": 12,
                    "qty_rows": 8,
                    "probe_locations": [
                        {
                            "data_column_index": 5,
                            "position_x": 140.8,
                            "position_y": 80,
                            "name": "Probe 5"
                        },
                        {
                            "data_column_index": 6,
                            "position_x": 103.1,
                            "position_y": 21.9,
                            "name": "Probe 6"
                        },
                        {
                            "data_column_index": 7,
                            "position_x": 48.1,
                            "position_y": 22.4,
                            "name": "Probe 7"
                        },
                        {
                            "data_column_index": 8,
                            "position_x": 7.2,
                            "position_y": 93.3,
                            "name": "Probe 8"
                        }
                    ],
                    "upper_left_corner_x": 536,
                    "upper_left_corner_y": 529,
                    "lower_right_corner_x": 823,
                    "lower_right_corner_y": 67,
                    "order_sequence": 2
                }
            ]
        })];

        let pb = ProgressBar::new(tray_configs.len() as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-"));

        for config_data in tray_configs {
            let name = config_data["name"].as_str().unwrap();
            pb.set_message(format!("Creating: {}", name));

            let result = self
                .make_request("POST", "/tray_configurations", Some(config_data))
                .await?;
            self.created_objects.tray_configurations.push(result);

            pb.inc(1);
            sleep(Duration::from_millis(200)).await;
        }

        pb.finish_with_message("Tray configurations created!");
        println!(
            "{} Created {} tray configurations",
            style("✅").green(),
            self.created_objects.tray_configurations.len()
        );

        Ok(())
    }

    pub async fn create_experiments(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} Creating experiments...", style("[6/7]").bold().dim());

        // Define tray regions with treatment boundaries and dilutions for the experiments
        let experiment_templates = vec![
            json!({
                "name": "Arctic Aerosol INP Characterization Exp139",
                "username": "researcher@eerl.lab",
                "temperature_ramp": -1.0,
                "temperature_start": 5.0,
                "temperature_end": -25.0,
                "is_calibration": false,
                "remarks": "Comprehensive characterization of Arctic atmospheric aerosol samples using SPICE droplet freezing technique",
                "regions": [
                    {
                        "name": "Untreated Samples - P1",
                        "display_colour_hex": "#3B82F6",
                        "tray_id": 1,
                        "col_min": 0, "col_max": 3, "row_min": 0, "row_max": 7,
                        "dilution_factor": 1,
                        "is_background_key": false
                    },
                    {
                        "name": "Heat Treated - P1",
                        "display_colour_hex": "#EF4444",
                        "tray_id": 1,
                        "col_min": 4, "col_max": 7, "row_min": 0, "row_max": 7,
                        "dilution_factor": 1,
                        "is_background_key": false
                    },
                    {
                        "name": "H2O2 Treated - P1",
                        "display_colour_hex": "#10B981",
                        "tray_id": 1,
                        "col_min": 8, "col_max": 11, "row_min": 0, "row_max": 7,
                        "dilution_factor": 1,
                        "is_background_key": false
                    },
                    {
                        "name": "Dilution Series 1:10 - P2",
                        "display_colour_hex": "#8B5CF6",
                        "tray_id": 2,
                        "col_min": 0, "col_max": 5, "row_min": 0, "row_max": 7,
                        "dilution_factor": 10,
                        "is_background_key": false
                    },
                    {
                        "name": "Dilution Series 1:100 - P2",
                        "display_colour_hex": "#F59E0B",
                        "tray_id": 2,
                        "col_min": 6, "col_max": 11, "row_min": 0, "row_max": 7,
                        "dilution_factor": 100,
                        "is_background_key": false
                    }
                ]
            }),
            json!({
                "name": "Agricultural Dust INP Analysis",
                "username": "agri.scientist@university.edu",
                "temperature_ramp": -0.5,
                "temperature_start": 2.0,
                "temperature_end": -20.0,
                "is_calibration": false,
                "remarks": "Investigation of ice nucleation activity in agricultural dust particles from Midwest farming regions",
                "regions": [
                    {
                        "name": "Control - P1",
                        "display_colour_hex": "#6B7280",
                        "tray_id": 1,
                        "col_min": 0, "col_max": 5, "row_min": 0, "row_max": 7,
                        "dilution_factor": 1,
                        "is_background_key": false
                    },
                    {
                        "name": "Agricultural Samples - P1",
                        "display_colour_hex": "#16A34A",
                        "tray_id": 1,
                        "col_min": 6, "col_max": 11, "row_min": 0, "row_max": 7,
                        "dilution_factor": 1,
                        "is_background_key": false
                    },
                    {
                        "name": "Heat Treatment - P2",
                        "display_colour_hex": "#DC2626",
                        "tray_id": 2,
                        "col_min": 0, "col_max": 11, "row_min": 0, "row_max": 7,
                        "dilution_factor": 1,
                        "is_background_key": false
                    }
                ]
            }),
            json!({
                "name": "Marine Boundary Layer INP Study",
                "username": "marine.researcher@oceaninst.org",
                "temperature_ramp": -1.2,
                "temperature_start": 8.0,
                "temperature_end": -28.0,
                "is_calibration": false,
                "remarks": "Characterization of marine-derived ice nucleation particles and their temperature-dependent activation",
                "regions": [
                    {
                        "name": "Marine Aerosols - P1",
                        "display_colour_hex": "#0EA5E9",
                        "tray_id": 1,
                        "col_min": 0, "col_max": 11, "row_min": 0, "row_max": 3,
                        "dilution_factor": 1,
                        "is_background_key": false
                    },
                    {
                        "name": "Size-Segregated Samples - P1",
                        "display_colour_hex": "#7C3AED",
                        "tray_id": 1,
                        "col_min": 0, "col_max": 11, "row_min": 4, "row_max": 7,
                        "dilution_factor": 1,
                        "is_background_key": false
                    },
                    {
                        "name": "Processed Controls - P2",
                        "display_colour_hex": "#EC4899",
                        "tray_id": 2,
                        "col_min": 0, "col_max": 11, "row_min": 0, "row_max": 7,
                        "dilution_factor": 1,
                        "is_background_key": false
                    }
                ]
            }),
        ];

        let pb = ProgressBar::new(experiment_templates.len() as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-"));

        for (i, mut template) in experiment_templates.into_iter().enumerate() {
            let name = template["name"].as_str().unwrap();
            pb.set_message(format!("Creating: {}", name));

            // Add realistic timestamp
            let performed_at = (Utc::now() - ChronoDuration::days((i as i64 * 7) + 2))
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string();
            template["performed_at"] = json!(performed_at);

            // Link to tray configuration (use first available)
            if let Some(tray_config) = self.created_objects.tray_configurations.first() {
                template["tray_configuration_id"] = json!(tray_config["id"].as_str().unwrap());
            }

            // Link regions to actual treatments if available
            if let Some(regions) = template.get_mut("regions") {
                if let Some(regions_array) = regions.as_array_mut() {
                    for region in regions_array {
                        // Find a suitable treatment based on region name
                        let region_name = region["name"].as_str().unwrap_or("");
                        let treatment_name = if region_name.to_lowercase().contains("heat") {
                            "heat"
                        } else if region_name.to_lowercase().contains("h2o2") {
                            "h2o2"
                        } else {
                            "none"
                        };

                        // Find a treatment with this name from our created treatments
                        if let Some(treatment) = self.created_objects.treatments.iter().find(|t| {
                            t.get("treatment_name").and_then(|n| n.as_str()) == Some(treatment_name)
                                || t.get("name").and_then(|n| n.as_str()) == Some(treatment_name)
                        }) {
                            region["treatment_id"] = json!(treatment["id"].as_str().unwrap());
                        }
                    }
                }
            }

            let result = self
                .make_request("POST", "/experiments", Some(template))
                .await?;
            self.created_objects.experiments.push(result);

            pb.inc(1);
            sleep(Duration::from_millis(100)).await;
        }

        pb.finish_with_message("Experiments created!");
        println!(
            "{} Created {} experiments",
            style("✅").green(),
            self.created_objects.experiments.len()
        );

        Ok(())
    }

    pub async fn process_excel_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{} Processing Excel file...", style("[7/7]").bold().dim());

        // Find the Excel file
        let excel_file_path = PathBuf::from("src/experiments/test_resources/merged.xlsx");
        if !excel_file_path.exists() {
            println!(
                "{} Excel file not found at {:?}, skipping processing",
                style("⚠️").yellow(),
                excel_file_path
            );
            return Ok(());
        }

        if self.created_objects.experiments.is_empty() {
            println!(
                "{} No experiments created, skipping Excel processing",
                style("⚠️").yellow()
            );
            return Ok(());
        }

        // Use the first experiment for Excel processing
        let experiment = &self.created_objects.experiments[0];
        let experiment_id = experiment["id"].as_str().unwrap();
        let experiment_name = experiment["name"].as_str().unwrap();

        println!(
            "{} Processing merged.xlsx for experiment: {}",
            style("📊").cyan(),
            style(experiment_name).bold()
        );

        // Show processing animation
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.blue} {msg}")
                .unwrap(),
        );
        pb.set_message("Uploading and processing Excel file...");
        pb.enable_steady_tick(Duration::from_millis(100));

        match self
            .make_multipart_request(
                &format!("/experiments/{}/process-excel", experiment_id),
                &excel_file_path,
            )
            .await
        {
            Ok(result) => {
                pb.finish_with_message("Excel processing completed!");

                self.created_objects.processed_experiments.push(json!({
                    "experiment_id": experiment_id,
                    "experiment_name": experiment_name,
                    "processing_result": result
                }));

                // Display processing results
                if let Some(temp_readings) = result.get("temperature_readings_created") {
                    println!(
                        "   {} Temperature readings: {}",
                        style("📊").cyan(),
                        style(temp_readings.as_u64().unwrap_or(0)).bold().green()
                    );
                }
                if let Some(phase_transitions) = result.get("phase_transitions_created") {
                    println!(
                        "   {} Phase transitions: {}",
                        style("🧊").cyan(),
                        style(phase_transitions.as_u64().unwrap_or(0))
                            .bold()
                            .green()
                    );
                }

                println!("{} Excel processing successful!", style("✅").green());
            }
            Err(e) => {
                pb.finish_with_message("Excel processing failed");
                println!("{} Excel processing failed: {}", style("⚠️").yellow(), e);
                println!(
                    "   This might be expected if tray configuration assignment is needed first"
                );
            }
        }

        Ok(())
    }

    pub async fn seed_database(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", style("SPICE Database Seeder").bold().blue());
        println!(
            "{}",
            style("Creating realistic scientific research data...").dim()
        );
        println!();

        // Execute seeding steps
        self.create_projects().await?;
        self.create_locations().await?;
        self.create_samples().await?;
        self.create_treatments().await?;
        self.create_tray_configurations().await?;
        self.create_experiments().await?;
        self.process_excel_file().await?;

        // Display summary
        self.display_summary();

        Ok(())
    }

    fn display_summary(&self) {
        println!();
        println!("{}", style("🎉 Database Seeding Complete!").bold().green());
        println!("{}", style("═".repeat(50)).dim());

        let summary_data = vec![
            ("Projects", self.created_objects.projects.len()),
            ("Locations", self.created_objects.locations.len()),
            ("Samples", self.created_objects.samples.len()),
            ("Treatments", self.created_objects.treatments.len()),
            (
                "Tray Configurations",
                self.created_objects.tray_configurations.len(),
            ),
            ("Experiments", self.created_objects.experiments.len()),
            (
                "Processed Experiments",
                self.created_objects.processed_experiments.len(),
            ),
        ];

        for (name, count) in summary_data {
            if count > 0 {
                println!(
                    "{:.<20} {}",
                    style(name).cyan(),
                    style(count).bold().green()
                );
            }
        }

        println!();
        println!("{} Next Steps:", style("🎯").cyan());
        println!("  {} Open SPICE UI to explore the data", style("•").dim());
        println!(
            "  {} Check experiment results and visualizations",
            style("•").dim()
        );
        println!(
            "  {} Verify Excel processing worked correctly",
            style("•").dim()
        );
        println!(
            "  {} Use this data for API testing and development",
            style("•").dim()
        );
        println!();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("SPICE Database Seeder")
        .version("1.0")
        .author("SPICE Development Team")
        .about("Seeds the SPICE database with realistic research data and processes Excel files")
        .arg(
            Arg::new("url")
                .short('u')
                .long("url")
                .value_name("URL")
                .help("API base URL")
                .default_value("http://localhost:3000"),
        )
        .arg(
            Arg::new("token")
                .short('t')
                .long("token")
                .value_name("JWT_TOKEN")
                .help("JWT authentication token")
                .required(true),
        )
        .get_matches();

    let base_url = matches.get_one::<String>("url").unwrap().clone();
    let jwt_token = matches.get_one::<String>("token").unwrap().clone();

    println!("{}", style("SPICE Database Seeder v1.0").bold());
    println!("{}", style("━".repeat(40)).dim());
    println!("API URL: {}", style(&base_url).cyan());
    println!(
        "Token:   {}...{}",
        style("*".repeat(8)).dim(),
        style(&jwt_token[jwt_token.len().saturating_sub(8)..]).dim()
    );

    let mut seeder = DatabaseSeeder::new(base_url, jwt_token);
    seeder.seed_database().await?;

    Ok(())
}
