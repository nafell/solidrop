use aws_sdk_s3::Client;

use crate::config::AppConfig;

pub async fn create_s3_client(config: &AppConfig) -> Client {
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(config.aws_region.clone()))
        .load()
        .await;

    let mut s3_config = aws_sdk_s3::config::Builder::from(&aws_config);

    if let Some(endpoint_url) = &config.s3_endpoint_url {
        s3_config = s3_config.endpoint_url(endpoint_url);
    }

    if config.s3_force_path_style {
        s3_config = s3_config.force_path_style(true);
    }

    Client::from_conf(s3_config.build())
}

/// Rewrite a presigned URL's host from the internal Docker endpoint to the public endpoint.
///
/// When running inside Docker, presigned URLs contain the internal hostname (e.g. `http://minio:9000`).
/// Clients outside Docker need URLs pointing to `http://localhost:9000` instead.
pub fn rewrite_presigned_url_for_public_access(
    url: &str,
    internal_endpoint: &str,
    public_endpoint: &str,
) -> String {
    url.replace(internal_endpoint, public_endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_presigned_url() {
        let url = "http://minio:9000/solidrop-dev/test.enc?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=minioadmin";
        let result = rewrite_presigned_url_for_public_access(
            url,
            "http://minio:9000",
            "http://localhost:9000",
        );
        assert_eq!(
            result,
            "http://localhost:9000/solidrop-dev/test.enc?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=minioadmin"
        );
    }

    #[test]
    fn test_rewrite_noop_when_no_match() {
        let url = "https://s3.amazonaws.com/bucket/key?signature=abc";
        let result = rewrite_presigned_url_for_public_access(
            url,
            "http://minio:9000",
            "http://localhost:9000",
        );
        assert_eq!(url, result);
    }
}
