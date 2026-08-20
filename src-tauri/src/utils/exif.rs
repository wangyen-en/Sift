// ============================================================
// Sift - EXIF Reading Utility
// ============================================================

use crate::models::photo::{Dimensions, ExifData};
use std::fs::{self, File};
use std::io::BufReader;

pub fn read_exif_data(jpg_path: &str) -> Result<ExifData, String> {
    let file = File::open(jpg_path).map_err(|e| format!("Failed to open file: {}", e))?;
    let file_size = fs::metadata(jpg_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let mut reader = BufReader::new(file);

    let exif_reader = exif::Reader::new();
    let exif = exif_reader
        .read_from_container(&mut reader)
        .map_err(|e| format!("Failed to read EXIF: {}", e))?;

    let camera = get_field_string(&exif, exif::Tag::Model);
    let lens = get_field_string(&exif, exif::Tag::LensModel);
    let iso = get_field_u32(&exif, exif::Tag::PhotographicSensitivity);
    let aperture = get_field_rational_string(&exif, exif::Tag::FNumber, "f/");
    let shutter_speed = get_shutter_speed(&exif);
    let focal_length = get_field_rational_string(&exif, exif::Tag::FocalLength, "");
    let date_taken = get_field_string(&exif, exif::Tag::DateTimeOriginal);

    let width = get_field_u32(&exif, exif::Tag::PixelXDimension);
    let height = get_field_u32(&exif, exif::Tag::PixelYDimension);

    let (latitude, longitude, altitude) = get_gps(&exif);

    Ok(ExifData {
        camera,
        lens,
        iso,
        aperture,
        shutter_speed,
        focal_length: if focal_length.is_empty() {
            String::new()
        } else {
            format!("{}mm", focal_length)
        },
        date_taken,
        dimensions: Dimensions { width, height },
        file_size,
        latitude,
        longitude,
        altitude,
    })
}

/// Read GPS coordinates from EXIF, converted to decimal degrees.
/// Returns (latitude, longitude, altitude) — all `None` if absent.
fn get_gps(exif: &exif::Exif) -> (Option<f64>, Option<f64>, Option<f64>) {
    let lat = get_gps_coord(exif, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef, 'S');
    let lon = get_gps_coord(exif, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef, 'W');
    let alt = exif
        .get_field(exif::Tag::GPSAltitude, exif::In::PRIMARY)
        .and_then(|f| match &f.value {
            exif::Value::Rational(v) => v.first().map(|r| r.to_f64()),
            _ => None,
        });
    (lat, lon, alt)
}

/// Read a GPS coordinate (degrees/minutes/seconds rationals) and apply its
/// hemisphere reference (e.g. 'S' or 'W' => negative).
fn get_gps_coord(
    exif: &exif::Exif,
    coord_tag: exif::Tag,
    ref_tag: exif::Tag,
    neg_ref: char,
) -> Option<f64> {
    let coord = exif
        .get_field(coord_tag, exif::In::PRIMARY)
        .and_then(|f| match &f.value {
            exif::Value::Rational(v) if v.len() >= 3 => {
                let deg = v[0].to_f64();
                let min = v[1].to_f64();
                let sec = v[2].to_f64();
                Some(deg + min / 60.0 + sec / 3600.0)
            }
            _ => None,
        })?;

    let is_negative = exif
        .get_field(ref_tag, exif::In::PRIMARY)
        .map(|f| f.display_value().to_string())
        .and_then(|s| s.trim().chars().next())
        .map(|c| c == neg_ref)
        .unwrap_or(false);

    Some(if is_negative { -coord } else { coord })
}

fn get_field_string(exif: &exif::Exif, tag: exif::Tag) -> String {
    exif.get_field(tag, exif::In::PRIMARY)
        .map(|f| f.display_value().to_string().trim_matches('"').to_string())
        .unwrap_or_default()
}

fn get_field_u32(exif: &exif::Exif, tag: exif::Tag) -> u32 {
    exif.get_field(tag, exif::In::PRIMARY)
        .and_then(|f| match &f.value {
            exif::Value::Short(v) => v.first().map(|&x| x as u32),
            exif::Value::Long(v) => v.first().copied(),
            _ => f.display_value().to_string().parse().ok(),
        })
        .unwrap_or(0)
}

fn get_field_rational_string(exif: &exif::Exif, tag: exif::Tag, prefix: &str) -> String {
    exif.get_field(tag, exif::In::PRIMARY)
        .map(|f| {
            let val = f.display_value().to_string();
            if val.is_empty() {
                String::new()
            } else {
                format!("{}{}", prefix, val)
            }
        })
        .unwrap_or_default()
}

fn get_shutter_speed(exif: &exif::Exif) -> String {
    exif.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY)
        .map(|f| {
            let val = f.display_value().to_string();
            if val.is_empty() {
                String::new()
            } else {
                format!("{}s", val)
            }
        })
        .unwrap_or_default()
}
