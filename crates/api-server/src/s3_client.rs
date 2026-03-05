use aws_sdk_s3::Client;

use crate::config::AppConfig;

/// Build an S3 client from the provided endpoint URL (or the AWS default if None).
async fn build_client(config: &AppConfig, endpoint_url: Option<&str>) -> Client {
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(config.aws_region.clone()))
        .load()
        .await;

    let mut s3_config = aws_sdk_s3::config::Builder::from(&aws_config);

    if let Some(url) = endpoint_url {
        s3_config = s3_config.endpoint_url(url);
    }

    if config.s3_force_path_style {
        s3_config = s3_config.force_path_style(true);
    }

    Client::from_conf(s3_config.build())
}

/// S3 client for regular API calls (ListObjects, HeadObject, etc.).
/// Uses the internal Docker endpoint (`S3_ENDPOINT_URL`) when configured.
pub async fn create_s3_client(config: &AppConfig) -> Client {
    build_client(config, config.s3_endpoint_url.as_deref()).await
}

/// S3 client used exclusively for generating presigned URLs.
///
/// Presigned URL signatures include the `host` header, so the client must be
/// configured with the endpoint that clients will actually connect to.
/// In local dev that is `S3_PUBLIC_ENDPOINT_URL` (e.g. `http://localhost:9000`);
/// in production it falls back to `S3_ENDPOINT_URL` or the AWS default.
pub async fn create_presigning_s3_client(config: &AppConfig) -> Client {
    let endpoint = config
        .s3_public_endpoint_url
        .as_deref()
        .or(config.s3_endpoint_url.as_deref());
    build_client(config, endpoint).await
}

#[cfg(test)]
mod tests {
    // rewrite_presigned_url_for_public_access was removed: the two-client
    // approach makes URL rewriting unnecessary.
}
