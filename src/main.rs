use std::io::{copy, Write};

use actix_multipart::{Field, Multipart};
use actix_web::{middleware, web, App, Error, HttpResponse, HttpServer};
use futures_util::TryStreamExt as _;
use std::fs::{create_dir_all, File};
use uuid::Uuid;

use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use s3::BucketConfiguration;

use zip::ZipArchive;

use std::sync::Arc;

const TMP_FOLDER: &str = "./tmp";
const BUCKET_NAME: &str = "staticmaps";

fn create_file_path(field: &mut Field) -> Arc<String> {
    let filename = field
        .content_disposition()
        .get_filename()
        .map_or_else(|| Uuid::new_v4().to_string(), sanitize_filename::sanitize);
    let filepath = Arc::new(format!("{TMP_FOLDER}/{filename}"));

    filepath
}

async fn save_to_disk(field: &mut Field, filepath: Arc<String>) -> Result<(), Error> {
    let mut f = web::block(move || File::create(filepath.as_ref())).await??;

    // Field in turn is stream of *Bytes* object
    while let Some(chunk) = field.try_next().await? {
        // filesystem operations are blocking, we have to use threadpool
        f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
    }
    Ok(())
}

async fn post_handler(mut payload: Multipart) -> Result<HttpResponse, Error> {
    // iterate over multipart stream
    while let Some(mut field) = payload.try_next().await? {
        let filepath = create_file_path(&mut field);

        let filepath_clone = filepath.clone();
        let filepath_clone2 = filepath.clone();

        save_to_disk(&mut field, filepath_clone).await?;

        // File::create is blocking operation, use threadpool

        let fname = std::path::Path::new(filepath_clone2.as_ref());
        let file = File::open(&fname).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };

            {
                let comment = file.comment();
                if !comment.is_empty() {
                    println!("File {} comment: {}", i, comment);
                }
            }

            if (*file.name()).ends_with('/') {
                println!("File {} extracted to \"{}\"", i, outpath.display());
                create_dir_all(&outpath).unwrap();
            } else {
                println!(
                    "File {} extracted to \"{}\" ({} bytes)",
                    i,
                    outpath.display(),
                    file.size()
                );
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        create_dir_all(p).unwrap();
                    }
                }
                let mut outfile = File::create(&outpath).unwrap();
                copy(&mut file, &mut outfile).unwrap();
            }
        }

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
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");

    create_dir_all(TMP_FOLDER)?;

    HttpServer::new(|| {
        App::new().wrap(middleware::Logger::default()).service(
            web::resource("/")
                .route(web::get().to(index))
                .route(web::post().to(post_handler)),
        )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
