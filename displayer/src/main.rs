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

#[macro_use]
extern crate lazy_static;

use std::{
    net::{TcpListener, TcpStream},
    io::{prelude::*, BufReader, Read},
    fs::File
};
use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract, 
    http::StatusCode, 
    response::{Html, Response}, 
    routing::get, 
    Router,
    Extension,
};
use std::{
    sync::Arc,
    net::SocketAddr,
};
// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use log::{info, warn};


/* SETTINGS */
const PORT: i32 = 8000;


#[derive(Template)]
#[template(path = "index.html")]
struct TemplateInformation<'a> {
    test: &'a String,
}

#[tokio::main]
async fn main() {
    let t = TemplateInformation{ test: &"Mattia".to_string() };

    let app = Router::new()
        .route("/", get(root));

    axum::Server::bind(&format!("0.0.0.0:{}", PORT).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
    info!("Listening on : localhost:{}", PORT);
}

async fn root(
    Extension(templateInformation): Extension<TemplateInformation<'_>>
) -> axum::response::Response {
    templateInformation.into_response()
}
