use crate::config::Config;
use crate::assets::models::Model as S3Assets;
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::{Client as S3Client, config::Region};
use axum::http::StatusCode;
use futures::stream::{self, StreamExt};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// In-memory S3 mock for testing - stores files as byte arrays in a `HashMap`
/// This provides fast, reliable testing without external dependencies
pub struct MockS3Store {
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MockS3Store {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn put_object(&self, key: &str, data: Vec<u8>) -> Result<(), String> {
        self.files
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {e}"))?
            .insert(key.to_string(), data);
        Ok(())
    }

    pub fn get_object(&self, key: &str) -> Result<Vec<u8>, String> {
        self.files
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {e}"))?
            .get(key)
            .cloned()
            .ok_or_else(|| format!("Object not found: {key}"))
    }

    pub fn delete_object(&self, key: &str) -> Result<(), String> {
        self.files
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {e}"))?
            .remove(key);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn list_objects(&self) -> Result<Vec<String>, String> {
        Ok(self
            .files
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {e}"))?
            .keys()
            .cloned()
            .collect())
    }
}

// Global mock store for tests - similar to how we handle the in-memory database
pub static MOCK_S3_STORE: LazyLock<MockS3Store> = LazyLock::new(MockS3Store::new);

/// Clear the mock S3 store - useful for test cleanup
#[allow(dead_code)]
pub fn clear_mock_s3_store() {
    if let Ok(mut files) = MOCK_S3_STORE.files.lock() {
        files.clear();
    }
}

pub async fn get_client(config: &Config) -> Arc<S3Client> {
    let region = Region::new("us-east-1");
    let credentials = Credentials::new(
        &config.s3_access_key,
        &config.s3_secret_key,
        None,
        None,
        "manual",
    );
    let shared_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region.clone())
        .credentials_provider(credentials)
        .endpoint_url(&config.s3_url)
        .load()
        .await;
    
    // Use path-style addressing for MinIO compatibility
    let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
        .force_path_style(true)
        .build();

    Arc::new(S3Client::from_conf(s3_config))
}

/// Ensure the S3 bucket exists, creating it if necessary
pub async fn ensure_bucket_exists(config: &Config) -> Result<(), String> {
    // Skip for tests (uses mock)
    if config.tests_running {
        return Ok(());
    }

    let client = get_client(config).await;
    let bucket = &config.s3_bucket_id;

    // Check if bucket exists by trying to list objects
    match client.list_objects_v2().bucket(bucket).send().await {
        Ok(_) => {
            // Bucket exists and is accessible
            Ok(())
        }
        Err(_) => {
            // Bucket doesn't exist or isn't accessible, try to create it
            match client.create_bucket().bucket(bucket).send().await {
                Ok(_) => {
                    println!("Created S3 bucket: {}", bucket);
                    Ok(())
                }
                Err(err) => Err(format!("Failed to create S3 bucket {}: {err}", bucket)),
            }
        }
    }
}

pub async fn delete_from_s3(s3_key: &str) -> Result<(), String> {
    let config = Config::from_env();

    // Use mock for tests
    if config.tests_running {
        return MOCK_S3_STORE.delete_object(s3_key);
    }

    let client = get_client(&config).await;
    let bucket = &config.s3_bucket_id;

    match client
        .delete_object()
        .bucket(bucket)
        .key(s3_key)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Failed to delete object from S3: {err}")),
    }
}

/// Mock-aware S3 `put_object` operation
pub async fn put_object_to_s3(s3_key: &str, data: Vec<u8>, config: &Config) -> Result<(), String> {
    // Use mock for tests
    if config.tests_running {
        return MOCK_S3_STORE.put_object(s3_key, data);
    }

    // Real S3 operation for production (bucket created at startup)
    let client = get_client(config).await;
    let body = aws_sdk_s3::primitives::ByteStream::from(data);

    match client
        .put_object()
        .bucket(&config.s3_bucket_id)
        .key(s3_key)
        .body(body)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Failed to upload object to S3: {err}")),
    }
}

/// Mock-aware S3 `get_object` operation
pub async fn get_object_from_s3(s3_key: &str, config: &Config) -> Result<Vec<u8>, String> {
    // Use mock for tests
    if config.tests_running {
        return MOCK_S3_STORE.get_object(s3_key);
    }

    // Real S3 operation for production
    let client = get_client(config).await;

    match client
        .get_object()
        .bucket(&config.s3_bucket_id)
        .key(s3_key)
        .send()
        .await
    {
        Ok(response) => {
            let body = response
                .body
                .collect()
                .await
                .map_err(|e| format!("Failed to read S3 object body: {e}"))?;
            Ok(body.into_bytes().to_vec())
        }
        Err(err) => Err(format!("Failed to get object from S3: {err}")),
    }
}

// New function: concurrently download assets from S3 with progress logging.
// Returns the TempDir (to keep files alive) and a vector of (original filename, file path).
#[allow(dead_code)]
pub async fn download_assets(
    assets: Vec<S3Assets>,
    config: &Config,
    s3_client: Arc<S3Client>,
) -> Result<(TempDir, Vec<(String, PathBuf)>), (StatusCode, String)> {
    let temp_dir =
        tempfile::tempdir().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let temp_path = temp_dir.path().to_owned();

    let download_futures = stream::iter(assets.into_iter().map(|asset| {
        let temp_dir_path = temp_path.clone();
        let s3_client = s3_client.clone();
        async move {
            let tmp_file_path = temp_dir_path.join(&asset.original_filename);
            let mut tmp_file = tokio::fs::File::create(&tmp_file_path)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let s3_response = s3_client
                .get_object()
                .bucket(&config.s3_bucket_id)
                .key(&asset.s3_key)
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("S3 download error for {}: {}", asset.s3_key, e),
                    )
                })?;
            let mut body_stream = s3_response.body.into_async_read();
            tokio::io::copy(&mut body_stream, &mut tmp_file)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            Ok((asset.original_filename, tmp_file_path))
        }
    }))
    .buffer_unordered(10) // Adjust concurrency as needed.
    .collect::<Vec<Result<(String, PathBuf), (StatusCode, String)>>>()
    .await;

    let mut asset_paths = Vec::new();
    for result in download_futures {
        match result {
            Ok(tuple) => asset_paths.push(tuple),
            Err(e) => return Err(e),
        }
    }
    Ok((temp_dir, asset_paths))
}
