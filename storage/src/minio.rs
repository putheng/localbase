use aws_credential_types::Credentials;
use aws_sdk_s3::{config::BehaviorVersion, config::Builder, config::Region, Client};

pub fn build_client(endpoint: &str, access_key: &str, secret_key: &str, region: &str) -> Client {
    let creds = Credentials::new(access_key, secret_key, None, None, "static");
    let config = Builder::new()
        .behavior_version(BehaviorVersion::latest())
        .endpoint_url(endpoint)
        .credentials_provider(creds)
        .region(Region::new(region.to_owned()))
        .force_path_style(true)
        .build();
    Client::from_conf(config)
}
