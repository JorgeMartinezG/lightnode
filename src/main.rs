mod errors;
mod layer;

use std::path::Path;

use actix_multipart::Multipart;
use actix_web::{middleware, web, App, Error, HttpResponse, HttpServer};
use futures_util::TryStreamExt as _;
use std::fs::create_dir;
use uuid::Uuid;

use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use s3::BucketConfiguration;

use self::layer::Layer;
use errors::AppError;

const TMP_FOLDER: &str = "./tmp";

async fn post_handler(mut payload: Multipart) -> Result<HttpResponse, AppError> {
    // iterate over multipart stream
    while let Some(mut field) = payload.try_next().await? {
        let layer_uuid = Uuid::new_v4().to_string();
        let fgb_path = Layer::new(layer_uuid).store(&mut field).await;

        println!("{:?}", fgb_path);

        /*
        // Instantiate bucket.
        let bucket = Bucket::new(
            BUCKET_NAME,
            Region::Custom {
                region: "".to_owned(),
                endpoint: "http://localhost:9000".to_owned(),
            },
            Credentials {
                access_key: Some("3MCNVMfGOIQnTJiP".to_owned()),
                secret_key: Some("LGz1geDQfBEJ6hTXTC2Y39zLJRJXBjVI".to_owned()),
                security_token: None,
                session_token: None,
                expiration: None,
            },
        )
        .expect("Could not create bucket instance")
        .with_path_style();

        let result = bucket.head_object("/").await;

        if result.is_err() {
            let create_result = Bucket::create_with_path_style(
                bucket.name.as_str(),
                bucket.region.clone(),
                bucket.credentials.clone(),
                BucketConfiguration::default(),
            )
            .await
            .expect("Could not create bucket");

            println!(
                "=== Bucket created\n{} - {} - {}",
                bucket.name, create_result.response_code, create_result.response_text
            );
        }

        let mut path = tokio::fs::File::open(filepath.clone().as_ref())
            .await
            .unwrap();
        let _status_code = bucket.put_object_stream(&mut path, "/path").await.unwrap();
        */
    }

    Ok(HttpResponse::Ok().into())
}

async fn index() -> HttpResponse {
    let html = r#"<html>
        <head><title>Upload Test</title></head>
        <body>
            <form target="/" method="post" enctype="multipart/form-data">
                <input type="file" multiple name="file"/>
                <button type="submit">Submit</button>
            </form>
        </body>
    </html>"#;

    HttpResponse::Ok().body(html)
}

#[actix_web::main]
async fn main() -> Result<(), AppError> {
    std::env::set_var("RUST_LOG", "debug");

    let tmp_path = Path::new(TMP_FOLDER);
    if tmp_path.exists() == false {
        create_dir(tmp_path).expect("Could not create temporal directory");
    }

    HttpServer::new(|| {
        App::new().wrap(middleware::Logger::default()).service(
            web::resource("/")
                .route(web::get().to(index))
                .route(web::post().to(post_handler)),
        )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}
