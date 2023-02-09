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
use std::{fs, env};
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
}

#[derive(Clone)]
struct Image {
    path: String,
    latitude_deg: f64,
    longitude_deg: f64,
}

async fn toDegrees(rationals: &exif::Value) -> f64 {
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
    let asset = match tokio::fs::read(format!("/media/mattia/Backup - Video/Europareise_2022/Fotos_Flurin/{}", filename)).await {
        Ok(a) => a,
        Err(_) => {
            return axum::http::StatusCode::NOT_FOUND.into_response();
        }
    };

    let mime = if let Some(mime) = mime_guess::from_path(filename).first_raw() {
        mime
    } else {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    };
    let res = ([("Content-Type", mime)], (&asset).to_vec()).into_response();
    res
}


#[tokio::main]
async fn main() {
    let args = Flags::parse();

    let mut images = Vec::new();

    let files = fs::read_dir(args.data).unwrap();
    for path in files {
        if path.as_ref().unwrap().path().extension().unwrap().to_ascii_uppercase() != "JPG" {
            continue
        }
        
        let file = fs::File::open(path.as_ref().unwrap().path()).unwrap();
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

        let longitude_deg = toDegrees(&exif::Value::Rational(longitude_vals.to_vec())).await;

        let latitude_vals = match exif.get_field(Tag::GPSLatitude, In::PRIMARY) {
            Some(field) => match field.value {
                exif::Value::Rational(ref rationals) => rationals,
                _ => unreachable!()
            },
            None => {
                continue;
            }
        };

        let latitude_deg = toDegrees(&exif::Value::Rational(latitude_vals.to_vec())).await;

        images.push(Image {
            path: format!("{}", path.as_ref().unwrap().path().display()),
            latitude_deg,
            longitude_deg
        });


        println!("Added file: {}", path
            .unwrap()
            .path()
            .display());
    }



    let t = PageTemplate{ images };

    let app = axum::Router::new()
        .route("/", axum::routing::get(root))
        .route("/src/assets/:filename", axum::routing::get(asset))
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