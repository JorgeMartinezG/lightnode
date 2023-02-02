use actix_multipart::MultipartError;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    ActixError(actix_web::Error),
    IOError(std::io::Error),
    OsString,
    ActixBlockingError(actix_web::error::BlockingError),
    ZipError(zip::result::ZipError),
    ShapefileError(shapefile::Error),
    ShpToGeotypesError,
    GeozeroError(geozero::error::GeozeroError),
}

impl From<shapefile::Error> for AppError {
    fn from(error: shapefile::Error) -> Self {
        AppError::ShapefileError(error)
    }
}

impl From<actix_web::Error> for AppError {
    fn from(error: actix_web::Error) -> Self {
        AppError::ActixError(error)
    }
}

impl From<MultipartError> for AppError {
    fn from(error: MultipartError) -> Self {
        AppError::ActixError(error.into())
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::IOError(error)
    }
}

impl From<std::ffi::OsString> for AppError {
    fn from(_error: std::ffi::OsString) -> Self {
        AppError::OsString
    }
}

impl From<actix_web::error::BlockingError> for AppError {
    fn from(error: actix_web::error::BlockingError) -> Self {
        AppError::ActixBlockingError(error)
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(error: zip::result::ZipError) -> Self {
        AppError::ZipError(error)
    }
}
impl From<geozero::error::GeozeroError> for AppError {
    fn from(error: geozero::error::GeozeroError) -> Self {
        AppError::GeozeroError(error)
    }
}

impl actix_web::error::ResponseError for AppError {}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::ActixError(error) => {
                formatter.write_str(format!("ACTIX::{}", error).as_str())
            }
            AppError::IOError(error) => formatter.write_str(format!("IO::{}", error).as_str()),
            AppError::OsString => formatter.write_str("OSSTRING::Failed to convert osstring"),
            AppError::ActixBlockingError(error) => {
                formatter.write_str(format!("ACTIX BLOCKING ERROR::{}", error).as_str())
            }
            AppError::ZipError(error) => formatter.write_str(format!("ZIP::{}", error).as_str()),
            AppError::ShapefileError(error) => {
                formatter.write_str(format!("SHP::{}", error).as_str())
            }
            AppError::ShpToGeotypesError => {
                formatter.write_str("SHP::Failed to convert shapefile feature to geo types")
            }
            AppError::GeozeroError(error) => {
                formatter.write_str(format!("SHP::{}", error).as_str())
            }
        }
    }
}
