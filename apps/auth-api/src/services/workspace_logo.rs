use std::io::Cursor;

use base64::{engine::general_purpose::STANDARD, Engine};
use image::{codecs::jpeg::JpegEncoder, ImageFormat, ImageReader, Limits};

use crate::errors::AppError;

pub fn normalize_jpeg(encoded: &str) -> Result<Vec<u8>, AppError> {
    let invalid = || AppError::Validation("Upload a valid workspace logo image".to_string());
    if encoded.len() > 60_000 {
        return Err(AppError::Validation(
            "The workspace logo is too large".to_string(),
        ));
    }

    let mut reader = ImageReader::with_format(
        Cursor::new(STANDARD.decode(encoded).map_err(|_| invalid())?),
        ImageFormat::Jpeg,
    );
    let mut limits = Limits::default();
    limits.max_image_width = Some(256);
    limits.max_image_height = Some(256);
    limits.max_alloc = Some(4 * 1024 * 1024);
    reader.limits(limits);

    let image = reader.decode().map_err(|_| invalid())?.into_rgb8();
    let mut logo = Vec::new();
    JpegEncoder::new_with_quality(&mut logo, 85)
        .encode_image(&image)
        .map_err(|err| AppError::Internal(format!("Could not encode workspace logo: {err}")))?;
    if logo.len() > 65_536 {
        return Err(AppError::Validation(
            "The workspace logo is too large".to_string(),
        ));
    }
    Ok(logo)
}
