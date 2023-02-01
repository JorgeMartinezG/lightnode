use std::io::{copy, BufWriter, Write};
use std::path::{Path, PathBuf};

use actix_multipart::{Field, Multipart};
use actix_web::{middleware, web, App, Error, HttpResponse, HttpServer};
use futures_util::TryStreamExt as _;
use std::fs::{create_dir, File};
use uuid::Uuid;

use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use s3::BucketConfiguration;

use flatgeobuf::FgbCrs;
use flatgeobuf::FgbWriter;
use flatgeobuf::FgbWriterOptions;
use flatgeobuf::GeometryType;

use shapefile::ShapeType;
use zip::ZipArchive;

const TMP_FOLDER: &str = "./tmp";

struct Layer {
    uuid: String,
    folder_path: PathBuf,
}

impl Layer {
    fn new(id: String) -> Layer {
        let folder_path = PathBuf::from(TMP_FOLDER).join(&id);

        let layer = Layer {
            uuid: id,
            folder_path: folder_path.clone(),
        };

        return layer;
    }

    fn create_folder(self) -> Self {
        create_dir(&self.folder_path).expect("Could not create layer directory");
        self
    }

    fn create_zip_path(&self) -> Box<Path> {
        let mut zip_file_path = self.folder_path.join(&self.uuid);
        zip_file_path.set_extension("zip");

        return zip_file_path.into_boxed_path();
    }

    fn create_shp_path(&self) -> Box<Path> {
        let mut shp_file_path = self.folder_path.join(&self.uuid);
        shp_file_path.set_extension("shp");

        return shp_file_path.into_boxed_path();
    }

    fn create_fgb_path(&self) -> Box<Path> {
        let mut fgb_file_path = self.folder_path.join(&self.uuid);
        fgb_file_path.set_extension("fgb");

        return fgb_file_path.into_boxed_path();
    }

    async fn save_zip_to_disk(self, field: &mut Field) -> Result<Self, Error> {
        let zip_file_path = self.create_zip_path();

        let mut f = web::block(move || File::create(zip_file_path)).await??;

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.try_next().await? {
            f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
        }

        Ok(self)
    }

    fn extract_zip(self) -> Self {
        let zip_file_path = self.create_zip_path();

        let file = File::open(zip_file_path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            println!("{:?}", file.enclosed_name());

            let outfile = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };

            let extension = String::from(outfile.extension().unwrap().to_str().unwrap());

            let outpath = self
                .folder_path
                .join(format!("{}.{}", self.uuid, extension));

            if (*file.name()).ends_with('/') {
                println!("File {} extracted to \"{}\"", i, outpath.display());
                create_dir(&outpath).unwrap();
            }
            println!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    create_dir(p).unwrap();
                }
            }
            let mut outfile = File::create(&outpath).unwrap();
            copy(&mut file, &mut outfile).unwrap();
        }

        self
    }

    fn to_geobuff(self) -> Self {
        let mut reader = shapefile::Reader::from_path(self.create_shp_path()).unwrap();
        let fgb_shape = match &reader.header().shape_type {
            ShapeType::Point => GeometryType::Point,
            ShapeType::Polygon => GeometryType::Polygon,
            _ => panic!("Shape not implemented!!"),
        };

        let mut fgb = FgbWriter::create_with_options(
            &self.uuid,
            fgb_shape,
            FgbWriterOptions {
                write_index: false,
                crs: FgbCrs {
                    code: 4326,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("flatgeoboom");

        for result in reader.iter_shapes_and_records() {
            let (shape, _records) = result.unwrap();
            let geometry = geo_types::Geometry::<f64>::try_from(shape).unwrap();

            fgb.add_feature_geom(geometry.clone(), |_feat| {}).unwrap();
        }

        let mut file = BufWriter::new(
            std::fs::File::create(self.create_fgb_path().as_os_str().to_str().unwrap()).unwrap(),
        );
        fgb.write(&mut file).unwrap();

        file.flush().unwrap();

        self
    }
}

async fn post_handler(mut payload: Multipart) -> Result<HttpResponse, Error> {
    // iterate over multipart stream
    while let Some(mut field) = payload.try_next().await? {
        let layer_uuid = Uuid::new_v4().to_string();
        Layer::new(layer_uuid)
            .create_folder()
            .save_zip_to_disk(&mut field)
            .await
            .expect("Could not create zip file")
            .extract_zip()
            .to_geobuff();

        //save_zip_to_disk(&mut field, &layer).await.unwrap();

        /*


        let shp_file_path = extract(&zip_file_path);

        println!("{:?}", zip_file_path);

        save_to_disk(&mut field, filepath.clone()).await?;
        extract_zip(filepath.clone());

        */

        // File::create is blocking operation, use threadpool

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
    .await
}
