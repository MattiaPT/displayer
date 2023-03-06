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
use std::{env, ffi::OsStr, fs, path::PathBuf};

use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{self, Extension},
    http::Response,
};
use chrono::{Duration, NaiveDateTime};
use clap::Parser;
use log::{error, info, warn};

use exif::{self, In, Tag};
use walkdir::WalkDir;

#[derive(Parser)]
struct Flags {
    #[clap(long)]
    port: u16,
    #[clap(long)]
    data: PathBuf,
}

#[derive(Template, Clone)]
#[template(path = "index.html")]
struct PageTemplate {
    images: Vec<Image>,
    first_date_time: i64,
    last_date_time: i64,
    delta: Duration,

    google_maps_api_key: String,
}

#[derive(Clone)]
struct Image {
    id: u64,
    path: String,
    image_date_time_naive: NaiveDateTime,
    latlon: LatLon,
}

#[derive(Clone)]
struct LatLon {
    latitude_deg: f64,
    longitude_deg: f64,
}

// TODO: fix path generation
const REPLACEMENT: &str = "slash";
const DELTA: i64 = 3;

async fn to_degrees(rationals: &exif::Value) -> f64 {
    let rationals = match *rationals {
        exif::Value::Rational(ref rationals) => rationals,
        _ => unreachable!(),
    };

    let mut total = 0.0;
    let mut weight = 1.0;
    for i in 0..3 {
        total += rationals[i].num as f64 / rationals[i].denom as f64 * weight;
        weight /= 60.0;
    }

    total
}

async fn asset_get(extract::Path(filename): extract::Path<String>) -> axum::response::Response {
    let f_name = str::replace(&filename, REPLACEMENT, "/");
    let asset = match tokio::fs::read(format!("{}", f_name)).await {
        Ok(a) => a,
        Err(e) => {
            warn!("Error reading file {}: {}", &filename, e);
            return axum::http::StatusCode::NOT_FOUND.into_response();
        }
    };

    let mime = match mime_guess::from_path(f_name).first_raw() {
        Some(m) => m,
        None => return axum::http::StatusCode::NOT_FOUND.into_response(),
    };

    ([("Content-Type", mime)], (&asset).to_vec()).into_response()
}

async fn src_get_css(extract::Path(filename): extract::Path<String>) -> impl IntoResponse {
    let markup = tokio::fs::read(format!("src/{}", filename)).await.unwrap();
    match std::path::Path::new(&filename)
        .extension()
        .unwrap()
        .to_str()
        .unwrap()
    {
        "ico" => ([("Content-Type", "image/x-icon")], markup).into_response(),
        "css" => Response::builder()
            .header("content-type", "text/css;charset=utf-8")
            .body(String::from_utf8(markup).unwrap())
            .unwrap()
            .into_response(),
        &_ => ().into_response(),
    }
}

async fn fetch_files(directory: &PathBuf) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    let directory_path: PathBuf =
        PathBuf::from(format!("{}", (*directory).clone().as_path().display()));
    for file in WalkDir::new(directory_path)
        .into_iter()
        .filter_map(|file| file.ok())
    {
        if !["JPG", "JPEG", "PNG"].iter().any(|extension| {
            file.path().extension() != None
                && OsStr::new(extension) == file.path().extension().unwrap().to_ascii_uppercase()
        }) {
            continue;
        }
        files.push(file.path().to_path_buf());
    }
    files
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Flags::parse();

    let mut images = Vec::new();

    let files = fetch_files(&args.data).await;
    let mut id: u64 = 0;
    for path in files.into_iter() {
        let file = match fs::File::open(path.as_path()) {
            Ok(f) => f,
            Err(e) => {
                warn!("Error reading path {}: {}", path.as_path().display(), e);
                continue;
            }
        };
        let mut bufreader = std::io::BufReader::new(&file);
        let exifreader = exif::Reader::new();
        let exif = match exifreader.read_from_container(&mut bufreader) {
            Ok(t) => t,
            Err(e) => {
                warn!(
                    "Error occured creating exif reader for {}: {}",
                    path.as_path().display(),
                    e
                );
                continue;
            }
        };

        let longitude_vals = match exif.get_field(Tag::GPSLongitude, In::PRIMARY) {
            Some(field) => match field.value {
                exif::Value::Rational(ref rationals) => rationals,
                _ => unreachable!(),
            },
            None => continue,
        };
        let longitude_deg = to_degrees(&exif::Value::Rational(longitude_vals.to_vec())).await;

        let latitude_vals = match exif.get_field(Tag::GPSLatitude, In::PRIMARY) {
            Some(field) => match field.value {
                exif::Value::Rational(ref rationals) => rationals,
                _ => unreachable!(),
            },
            None => continue,
        };
        let latitude_deg = to_degrees(&exif::Value::Rational(latitude_vals.to_vec())).await;

        let date_time_original = match exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
            Some(field) => match field.value {
                exif::Value::Ascii(ref ascii) => ascii,
                _ => unreachable!(),
            },
            None => continue,
        };
        let image_date_time_naive = match NaiveDateTime::parse_from_str(
            &String::from_utf8(date_time_original[0].clone()).unwrap(),
            "%Y:%m:%d %H:%M:%S",
        ) {
            Ok(t) => t,
            Err(e) => {
                error!("Error occurred parsing NaiveDateTime from str: {:?}", e);
                std::process::exit(1)
            }
        };

        id += 1;
        images.push(Image {
            id,
            path: str::replace(&format!("{}", path.as_path().display()), "/", REPLACEMENT),
            image_date_time_naive,
            latlon: LatLon {
                latitude_deg,
                longitude_deg,
            },
        });
    }
    info!("Displaying {} files", id);

    let first_date_time = match images
        .iter()
        .map(|image| image.image_date_time_naive.timestamp())
        .min()
    {
        Some(dt) => dt,
        None => {
            error!("Error occurred fetching first_date_time");
            std::process::exit(1)
        }
    };
    let last_date_time = match images
        .iter()
        .map(|image| image.image_date_time_naive.timestamp())
        .max()
    {
        Some(dt) => dt,
        None => {
            error!("Error occurred fetching last_date_time");
            std::process::exit(1)
        }
    };

    images.sort_by_key(|a| a.image_date_time_naive.timestamp());

    let template = PageTemplate {
        images,
        first_date_time,
        last_date_time,
        delta: Duration::days(DELTA),
        google_maps_api_key: env::var("GOOGLE_MAPS_API_KEY").unwrap(),
    };

    let app = axum::Router::new()
        .route("/", axum::routing::get(root))
        .route("/assets/:filename", axum::routing::get(asset_get))
        .route("/src/:filename", axum::routing::get(src_get_css))
        .layer(Extension(template.clone()));

    info!("Listening on: http://localhost:{}", args.port);
    axum::Server::bind(&format!("0.0.0.0:{}", args.port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root(Extension(images): Extension<PageTemplate>) -> axum::response::Response {
    images.into_response()
}
