/*
 * Author: Mattia
 * Date: 22.03.2023
 *
 * Reworked displayer application.
 *
 */

use std::{env, fs, io::BufReader, path::PathBuf};

use chrono::NaiveDateTime;
use clap::Parser;
use log::{info, warn};

use exif::{In, Reader, Tag};

async fn fetch_files(directory: &PathBuf) -> Vec<PathBuf> {
    vec![]
}

#[derive(Parser)]
struct Flags {
    #[clap(long)]
    port: u16,
    #[clap(long)]
    data: PathBuf,
}

#[derive(Clone)]
struct Image {
    id: u64,
    path: String,
    image_date_time: NaiveDateTime,
    latlonalt: LatLonAlt,
}

#[derive(Clone)]
struct LatLonAlt {
    latitude_deg: f64,
    longitude_deg: f64,
    altitude_m: i32,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Flags::parse();

    let files = fetch_files(&args.data).await;
    for path in files.into_iter() {
        let file = match fs::File::open(path.as_path()) {
            Ok(f) => f,
            Err(e) => {
                warn!("Error opening {}: {}", path.as_path().display(), e);
                continue;
            }
        };

        let mut bufreader = BufReader::new(&file);
        let exifreader = Reader::new();
        let exif = match exifreader.read_from_container(&mut bufreader) {
            Ok(t) => t,
            Err(e) => {
                warn!(
                    "Error reading or parsing attributes for {}: {}",
                    path.as_path().display(),
                    e
                );
                continue;
            }
        };

        let longitude_vec = match exif.get_field(Tag::GPSLongitude, In::PRIMARY) {
            Some(field) => match field.value {
                exif::Value::Rational(ref rationals) => rationals,
                _ => unreachable!(),
            },
            None => continue,
        };

        let latitude_vec = match exif.get_field(Tag::GPSLatitude, In::PRIMARY) {
            Some(field) => match field.value {
                exif::Value::Rational(ref rationals) => rationals,
                _ => unreachable!(),
            },
            None => continue,
        };

        let date_time_original = match exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
            Some(field) => match field.value {
                exif::Value::Ascii(ref ascii) => ascii,
                _ => unreachable!(),
            },
            None => continue,
        };
    }

    println!("Hello, world!");
}
