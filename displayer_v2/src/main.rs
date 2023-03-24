/*
 * Author: Mattia
 * Date: 22.03.2023
 *
 * Reworked displayer application.
 *
 */

use std::{env, ffi::OsStr, fs, io::BufReader, path::PathBuf};

use chrono::NaiveDateTime;
use clap::Parser;
use futures::future::{BoxFuture, FutureExt};
use log::{info, warn};

use exif::{In, Reader, Tag};

#[derive(Parser)]
struct Flags {
    #[clap(long)]
    port: u16,
    #[clap(long)]
    data: PathBuf,
}

#[derive(Clone, Debug)]
struct Image {
    id: u64,
    path: PathBuf,
    image_date_time: NaiveDateTime,
    latlonalt: LatLonAlt,
}

#[derive(Clone, Debug)]
struct LatLonAlt {
    latitude_deg: f64,
    longitude_deg: f64,
    altitude_m: i32,
}

const IMAGE_TYPES: [&str; 3] = ["JPG", "JPEG", "PNG"];

fn fetch_files(directory: &PathBuf, files: &mut Vec<PathBuf>) {
    async move {
        for entry in directory.read_dir().unwrap() {
            if let Ok(entry) = entry {
                let path = PathBuf::from(entry.path());
                if path.is_dir() {
                    fetch_files(&path, files);
                } else if IMAGE_TYPES.iter().any(|extension| {
                    path.extension() != None && path.extension().unwrap() == OsStr::new(extension)
                }) {
                    files.push(path);
                }
            }
        }
    }
    .boxed();
}

async fn rationals_to_degrees(rationals: &exif::Value) -> f64 {
    let rationals = match *rationals {
        exif::Value::Rational(ref rationals) => rationals,
        _ => unreachable!(),
    };

    let mut total = 0.0;
    let mut weight = 1.0;
    for i in 0..3 {
        total += (rationals[i].num as f64) / (rationals[i].denom as f64) * weight;
        weight /= 60.0;
    }

    total
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Flags::parse();

    let mut images: Vec<Image> = Vec::new();

    let mut files: Vec<PathBuf> = Vec::new();
    fetch_files(&args.data, &mut files);
    for path in files.into_iter() {
        let debug_info = path.as_path().display();

        let file = match fs::File::open(path.as_path()) {
            Ok(f) => f,
            Err(e) => {
                warn!("Error opening {}: {}", debug_info, e);
                continue;
            }
        };

        let mut bufreader = BufReader::new(&file);
        let exifreader = Reader::new();
        let exif = match exifreader.read_from_container(&mut bufreader) {
            Ok(t) => t,
            Err(e) => {
                warn!("Error reading/parsing attributes for {}: {}", debug_info, e);
                continue;
            }
        };

        let longitude_vec = match exif.get_field(Tag::GPSLongitude, In::PRIMARY) {
            Some(field) => &field.value,
            None => continue,
        };
        let latitude_vec = match exif.get_field(Tag::GPSLatitude, In::PRIMARY) {
            Some(field) => &field.value,
            None => continue,
        };
        let date_time_original = match exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
            Some(field) => match &field.value {
                exif::Value::Ascii(ref ascii) => String::from_utf8(ascii[0].clone()).unwrap(),
                _ => unreachable!(),
            },
            None => continue,
        };
        let image_date_time =
            match NaiveDateTime::parse_from_str(&date_time_original, "%Y:%m:%d %H:%M:%S") {
                Ok(t) => t,
                Err(e) => {
                    warn!("Error parsing NaiveDateTime for {}: {}", debug_info, e);
                    continue;
                }
            };

        // FIXME: altitude_m
        let latlonalt = LatLonAlt {
            latitude_deg: rationals_to_degrees(&latitude_vec).await,
            longitude_deg: rationals_to_degrees(&longitude_vec).await,
            altitude_m: 0,
        };
        images.push(Image {
            id: 0,
            path,
            image_date_time,
            latlonalt,
        });
    }

    println!("{:#?}", images);
}
