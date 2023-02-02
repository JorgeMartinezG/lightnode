use crate::errors::AppError;
use crate::layer::Layer;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use s3::BucketConfiguration;

pub struct S3Client {
    bucket: Bucket,
}

impl S3Client {
    pub fn new() -> Result<Self, AppError> {
        let region = Region::Custom {
            region: "".to_owned(),
            endpoint: "http://localhost:9000".to_owned(),
        };

        let credentials = Credentials {
            access_key: Some("7nMHoXVejZaupRtx".to_owned()),
            secret_key: Some("R3LylZkG08rX5plFNec9jvviJ8Y5MWoy".to_owned()),
            security_token: None,
            session_token: None,
            expiration: None,
        };

        let bucket = Bucket::new("layers", region, credentials)?.with_path_style();

        let client = Self { bucket: bucket };

        Ok(client)
    }

    pub async fn create_bucket_if_not_exists(self) -> Result<Self, AppError> {
        let result = self.bucket.head_object("/").await;

        let bucket = &self.bucket;

        if result.is_err() {
            let create_result = Bucket::create_with_path_style(
                bucket.name.as_str(),
                bucket.region.clone(),
                bucket.credentials.clone(),
                BucketConfiguration::default(),
            )
            .await?;

            println!(
                "=== Bucket created\n{} - {} - {}",
                bucket.name, create_result.response_code, create_result.response_text
            );
        }

        Ok(self)
    }

    pub async fn upload(self, layer: &Layer) -> Result<u16, AppError> {
        let fgb_path = layer.get_fgb_path()?;
        let mut path = tokio::fs::File::open(fgb_path).await?;
        let filename = format!("{}.fgb", layer.uuid);
        let status_code = self.bucket.put_object_stream(&mut path, filename).await?;

        Ok(status_code)
    }
}
