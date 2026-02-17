use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub s3_bucket: String,
    pub api_key: String,
    pub aws_region: String,
    /// Custom S3 endpoint URL for MinIO/LocalStack (e.g. "http://minio:9000")
    pub s3_endpoint_url: Option<String>,
    /// Use path-style addressing (required for MinIO)
    pub s3_force_path_style: bool,
    /// Public endpoint URL for presigned URLs (e.g. "http://localhost:9000")
    pub s3_public_endpoint_url: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            s3_bucket: env::var("S3_BUCKET").expect("S3_BUCKET must be set"),
            api_key: env::var("API_KEY").expect("API_KEY must be set"),
            aws_region: env::var("AWS_REGION").unwrap_or_else(|_| "ap-northeast-1".into()),
            s3_endpoint_url: env::var("S3_ENDPOINT_URL").ok(),
            s3_force_path_style: env::var("S3_FORCE_PATH_STYLE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            s3_public_endpoint_url: env::var("S3_PUBLIC_ENDPOINT_URL").ok(),
        }
    }
}
