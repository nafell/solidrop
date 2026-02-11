use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub s3_bucket: String,
    pub api_key: String,
    pub aws_region: String,
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
        }
    }
}
