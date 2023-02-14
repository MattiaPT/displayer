/*
    Author: Mattia
    Date: 05.02.2023

    This file should contain the main part of a imaging application
    displaying a map where images can be found wherever they were
    taken at.

    useful links:
    - https://tera.netlify.app/docs/
    - https://doc.rust-lang.org/book/ch20-01-single-threaded.html
    - https://github.com/Keats/tera/tree/master/examples

    - https://developers.google.com/maps/documentation/javascript/examples/overlay-simple#maps_overlay_simple-javascript
*/
use std::path::PathBuf;
use std::fs::metadata;
use std::ffi::OsStr;
use std::{fs, env};
use std::fs::DirEntry;
use log::{info, warn};
use exif::{self, Tag, In, Rational};

use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{self, Extension};
use clap::Parser;
use tokio::sync::Notify;

use axum::response::Response;
use mime_guess::mime;
use std::fs::File;
use std::io::Read;
use walkdir::WalkDir;
use chrono::{offset::TimeZone, NaiveDateTime, Local, DateTime, Datelike, Date, Duration};

#[derive(Parser)]
struct Flags {
    #[clap(long)]
    port: u16,
    #[clap(long)]
    data: PathBuf,
    #[clap(long)]
    delta: i64,
}

#[derive(Template, Clone)]
#[template(path = "index.html")]
struct PageTemplate {
    images: Vec<Image>,
    first_date_time: Date<Local>,
    last_date_time: Date<Local>,
    delta: Duration,
}

#[derive(Clone)]
struct Image {
    id: u64,
    path: String,
    image_date_time: Date<Local>,
    latitude_deg: f64,
    longitude_deg: f64,
}

async fn to_degrees(rationals: &exif::Value) -> f64 {
    let rationals = match *rationals {
        exif::Value::Rational(ref rationals) => rationals,
        _ => unreachable!()
    };

    let mut total = 0.0;
    let mut weight = 1.0;
    for i in 0..3 {
        total += rationals[i].num as f64 / rationals[i].denom as f64 * weight;
        weight /= 60.0;
    }

    total
}

async fn asset(
    extract::Path(filename): extract::Path<String>,
) -> axum::response::Response {
    let f_name = str::replace(&filename, "slash", "/");
    let asset = match tokio::fs::read(format!("{}", f_name)).await {
        Ok(a) => a,
        Err(_) => {
            return axum::http::StatusCode::NOT_FOUND.into_response();
        }
    };

    let mime = if let Some(mime) = mime_guess::from_path(f_name).first_raw() {
        mime
    } else {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    };
    let res = ([("Content-Type", mime)], (&asset).to_vec()).into_response();
    res
}

async fn fetch_files(directory: &PathBuf) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = Vec::new();
        let directory_path: PathBuf = PathBuf::from(format!("{}", (*directory).clone().as_path().display()));
        for file in WalkDir::new(directory_path).into_iter().filter_map(|file| file.ok()) {
            if !["JPG", "JPEG", "PNG"].iter().any(|extension| file.path().extension() != None && OsStr::new(extension) == file.path().extension().unwrap().to_ascii_uppercase()) {
                continue;
            }
            files.push(file.path().to_path_buf());
        }
        files
}

#[tokio::main]
async fn main() {
    let args = Flags::parse();

    let mut images = Vec::new();

    let files = fetch_files(&args.data).await;
    let mut i: u64 = 0;
    for path in files.into_iter() {
        print!("\rDisplaying {} files", i);
        let file = fs::File::open(path.as_path()).unwrap();
        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();
        let exif = match exifreader.read_from_container(&mut bufreader) {
            Ok(t) => t,
            Err(_) => {
                continue;
            }
        };

        let longitude_vals = match exif.get_field(Tag::GPSLongitude, In::PRIMARY) {
            Some(field) => match field.value {
                exif::Value::Rational(ref rationals) => rationals,
                _ => unreachable!()
            },
            None => {
                continue;
            }
        };
        let date_time_original = match exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
            Some(field) => match field.value {
                exif::Value::Ascii(ref ascii) => ascii,
                _ => unreachable!()
            },
            None => continue
        };
        let image_date_time_naive = &match NaiveDateTime::parse_from_str(&String::from_utf8(date_time_original[0].clone()).unwrap(), "%Y:%m:%d %H:%M:%S") {
            Ok(t) => t,
            Err(e) => {
                println!("{:?}", e);
                std::process::exit(1)
            }
        };
        let image_date_time = Local.from_local_datetime(image_date_time_naive).unwrap().date();

        let longitude_deg = to_degrees(&exif::Value::Rational(longitude_vals.to_vec())).await;

        let latitude_vals = match exif.get_field(Tag::GPSLatitude, In::PRIMARY) {
            Some(field) => match field.value {
                exif::Value::Rational(ref rationals) => rationals,
                _ => unreachable!()
            },
            None => {
                continue;
            }
        };

        let latitude_deg = to_degrees(&exif::Value::Rational(latitude_vals.to_vec())).await;

        i += 1;
        images.push(Image {
            id: i,
            path: str::replace(&format!("{}", path.as_path().display()), "/", "slash"),
            image_date_time,
            latitude_deg,
            longitude_deg
        });


        info!("Added file: {}", path
            .as_path()
            .display());
    }

    let first_date_time = images.iter().map(|image| Local.from_local_datetime(&image.image_date_time.naive_utc().and_hms_opt(0, 0, 0).unwrap()).unwrap().timestamp()).min().unwrap();
    let last_date_time = images.iter().map(|image| Local.from_local_datetime(&image.image_date_time.naive_utc().and_hms_opt(0, 0, 0).unwrap()).unwrap().timestamp()).max().unwrap();

    let t = PageTemplate{ 
        images,
        first_date_time: Local.from_local_datetime(&NaiveDateTime::from_timestamp_opt(first_date_time, 0).unwrap()).unwrap().date(),
        last_date_time: Local.from_local_datetime(&NaiveDateTime::from_timestamp_opt(last_date_time, 0).unwrap()).unwrap().date(),
        delta: Duration::days(args.delta)
    };

    let app = axum::Router::new()
        .route("/", axum::routing::get(root))
        .route("/assets/:filename", axum::routing::get(asset))
        .layer(Extension(t.clone()));

    axum::Server::bind(&format!("0.0.0.0:{}", args.port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
    println!("Listening on : localhost:{}", args.port);
}

async fn root(
    Extension(images): Extension<PageTemplate>,
) -> axum::response::Response {
    images.into_response()
}
