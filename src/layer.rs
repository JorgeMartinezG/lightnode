use crate::TMP_FOLDER;
use std::io::{copy, BufWriter, Write};
use std::path::{Path, PathBuf};

use actix_multipart::Field;
use actix_web::{web, Error};
use futures_util::TryStreamExt as _;
use std::fs::{create_dir, File};

use flatgeobuf::FgbCrs;
use flatgeobuf::FgbWriter;
use flatgeobuf::FgbWriterOptions;
use flatgeobuf::GeometryType;

use shapefile::ShapeType;
use zip::ZipArchive;

pub struct Layer {
    uuid: String,
    folder_path: PathBuf,
}

enum FileExtension {
    Shapefile,
    Flatgeobuf,
    Zip,
}

impl FileExtension {
    fn to_string(&self) -> String {
        match self {
            FileExtension::Shapefile => String::from("shp"),
            FileExtension::Flatgeobuf => String::from("fgb"),
            FileExtension::Zip => String::from("zip"),
        }
    }
}

impl Layer {
    pub fn new(id: String) -> Layer {
        let folder_path = PathBuf::from(TMP_FOLDER).join(&id);

        let layer = Layer {
            uuid: id,
            folder_path: folder_path.clone(),
        };

        return layer;
    }

    pub async fn store(self, field: &mut Field) -> String {
        self.create_folder()
            .save_zip_to_disk(field)
            .await
            .expect("Could not create zip file")
            .extract_zip()
            .to_geobuff()
    }

    fn create_folder(self) -> Self {
        create_dir(&self.folder_path).expect("Could not create layer directory");
        self
    }

    fn create_path(&self, extension: FileExtension) -> String {
        let mut path = self.folder_path.join(&self.uuid);
        let extension = extension.to_string();

        path.set_extension(extension);

        return path
            .into_os_string()
            .into_string()
            .expect("Could not transform into String");
    }

    async fn save_zip_to_disk(self, field: &mut Field) -> Result<Self, Error> {
        let zip_file_path = self.create_path(FileExtension::Zip);
        let mut f = web::block(move || File::create(zip_file_path)).await??;

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.try_next().await? {
            f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
        }

        Ok(self)
    }

    fn extract_zip(self) -> Self {
        let zip_file_path = self.create_path(FileExtension::Zip);

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

    fn to_geobuff(self) -> String {
        let shp_file_path = self.create_path(FileExtension::Shapefile);
        let mut reader = shapefile::Reader::from_path(shp_file_path).unwrap();
        let fgb_path = self.create_path(FileExtension::Flatgeobuf);

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

        let mut file = BufWriter::new(std::fs::File::create(&fgb_path).unwrap());
        fgb.write(&mut file).unwrap();
        file.flush().unwrap();

        fgb_path
    }
}
