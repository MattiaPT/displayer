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

use tera::{Context, Tera};
use std::{
    net::{TcpListener, TcpStream},
    io::{prelude::*, BufReader, Read},
    fs::File
};
use image::{GenericImageView, ImageFormat};


/* SETTINGS */
const PORT: i32 = 8000;


lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let tera = match Tera::new("src/templates/**/*.html") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera
    };
}

fn main() {

    let mut context = Context::new();
/* example:
    let name = "Mattia";
    context.insert("name", &name);
*/
    let contents = match TEMPLATES.render("index.html", &context) {
        Ok(s) => s,
        Err(e) => {
            println!("Error: {}", e);
            ::std::process::exit(1);
        }
    };


    let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT)).unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream, &contents);
    }
}


fn handle_connection(mut stream: TcpStream, contents: &String) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let errorString = "404 Not Found".to_string();
    let mut string = String::new();

    if http_request.len() == 0 {
        let response = format!("HTTP/1.1 200 OK\r\nContentLength: 0\r\n\r\n{}", contents);
        stream.write_all(response.as_bytes()).unwrap();
        return;
    }
    let (status_line, response_body) = match http_request[0].split_whitespace().nth(1).unwrap() {
        "/" => ("HTTP/1.1 200 OK", contents),
        "/assets/image" => {
            let mut buf = Vec::new();
            let mut file = File::open("src/assets/IMG_9147.JPG").unwrap();
            file.read_to_end(&mut buf).unwrap();
            let image = image::load_from_memory(&buf).unwrap();
            let format = ImageFormat::JPEG;

            /**/
            ("HTTP/1.1 200 OK", &string)
        },
        _ => ("HTTP/1.1 404 Not Found", &errorString),
    };

    let length = response_body.len();
    let response = format!("HTTP/1.1 200 OK\r\nContentLength: {}\r\n\r\n{}", length, response_body);

    // println!("Request: {:#?}", http_request);
    // println!("Response: {:#?}", response);

    stream.write_all(response.as_bytes()).unwrap();
}