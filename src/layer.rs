use crate::TMP_FOLDER;
use std::io::{copy, BufWriter, Write};
use std::path::PathBuf;

use actix_multipart::Field;
use actix_web::web;
use futures_util::TryStreamExt as _;
use std::fs::{create_dir, remove_dir_all, File};

use flatgeobuf::FgbCrs;
use flatgeobuf::FgbWriter;
use flatgeobuf::FgbWriterOptions;
use flatgeobuf::GeometryType;

use shapefile::ShapeType;
use uuid::Uuid;
use zip::ZipArchive;

use crate::errors::AppError;

pub struct Layer {
    pub uuid: String,
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
    pub fn new() -> Layer {
        let layer_uuid = Uuid::new_v4().to_string();
        let folder_path = PathBuf::from(TMP_FOLDER).join(&layer_uuid);

        let layer = Layer {
            uuid: layer_uuid,
            folder_path: folder_path.clone(),
        };

        return layer;
    }

    pub async fn store(self, field: &mut Field) -> Result<Self, AppError> {
        Ok(self
            .create_folder()?
            .save_zip_to_disk(field)
            .await?
            .extract_zip()?
            .to_geobuff()?)
    }

    fn create_folder(self) -> Result<Self, AppError> {
        create_dir(&self.folder_path)?;
        Ok(self)
    }

    pub fn delete_folder(self) -> Result<Self, AppError> {
        remove_dir_all(&self.folder_path)?;
        Ok(self)
    }

    fn create_path(&self, extension: FileExtension) -> Result<String, AppError> {
        let mut path = self.folder_path.join(&self.uuid);
        let extension = extension.to_string();

        path.set_extension(extension);

        let str_path = path.into_os_string().into_string()?;

        return Ok(str_path);
    }

    pub fn get_fgb_path(&self) -> Result<String, AppError> {
        let fgb_path = self.create_path(FileExtension::Flatgeobuf)?;

        Ok(fgb_path)
    }

    async fn save_zip_to_disk(self, field: &mut Field) -> Result<Self, AppError> {
        let zip_file_path = self.create_path(FileExtension::Zip)?;
        let mut f = web::block(move || File::create(zip_file_path)).await??;

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.try_next().await? {
            f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
        }

        Ok(self)
    }

    fn extract_zip(self) -> Result<Self, AppError> {
        let zip_file_path = self.create_path(FileExtension::Zip)?;

        let file = File::open(zip_file_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            println!("{:?}", file.enclosed_name());

            let outfile = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };

            let extension = outfile
                .extension()
                .map_or("", |ext| ext.to_str().map_or("", |ext| ext));

            let outpath = self
                .folder_path
                .join(format!("{}.{}", self.uuid, extension));

            if (*file.name()).ends_with('/') {
                println!("File {} extracted to \"{}\"", i, outpath.display());
                create_dir(&outpath)?;
            }
            println!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    create_dir(p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            copy(&mut file, &mut outfile)?;
        }

        Ok(self)
    }

    fn to_geobuff(self) -> Result<Self, AppError> {
        let shp_file_path = self.create_path(FileExtension::Shapefile)?;
        let mut reader = shapefile::Reader::from_path(shp_file_path)?;
        let fgb_path = self.create_path(FileExtension::Flatgeobuf)?;

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
        )?;

        for result in reader.iter_shapes_and_records() {
            let (shape, _records) = result?;
            let geometry = geo_types::Geometry::<f64>::try_from(shape)
                .map_err(|_| AppError::ShpToGeotypesError)?;

            fgb.add_feature_geom(geometry.clone(), |_feat| {})?;
        }

        let mut file = BufWriter::new(std::fs::File::create(&fgb_path)?);
        fgb.write(&mut file)?;
        file.flush()?;

        Ok(self)
    }
}
