use aws_sdk_s3::Client;

use crate::config::AppConfig;

pub async fn create_s3_client(config: &AppConfig) -> Client {
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(config.aws_region.clone()))
        .load()
        .await;
    Client::new(&aws_config)
}
