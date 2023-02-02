use actix_multipart::MultipartError;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    ActixError(MultipartError),
    IOError(std::io::Error),
}

impl From<MultipartError> for AppError {
    fn from(error: MultipartError) -> Self {
        AppError::ActixError(error)
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::IOError(error)
    }
}

impl actix_web::error::ResponseError for AppError {}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::ActixError(msg) => formatter.write_str(format!("ACTIX::{}", msg).as_str()),
            AppError::IOError(msg) => formatter.write_str(format!("IO::{}", msg).as_str()),
        }
    }
}
