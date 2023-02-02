mod client;
mod errors;
mod layer;

use std::path::Path;

use actix_multipart::Multipart;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use futures_util::TryStreamExt as _;
use std::fs::create_dir;

use crate::client::S3Client;
use crate::errors::AppError;
use crate::layer::Layer;

const TMP_FOLDER: &str = "./tmp";

async fn post_handler(mut payload: Multipart) -> Result<HttpResponse, AppError> {
    // iterate over multipart stream
    while let Some(mut field) = payload.try_next().await? {
        let client = S3Client::new()?.create_bucket_if_not_exists().await?;

        let layer = Layer::new().store(&mut field).await?;

        let status_code = client.upload(&layer).await?;

        layer.delete_folder()?;

        println!("{:?}", status_code);
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
        create_dir(tmp_path)?;
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
