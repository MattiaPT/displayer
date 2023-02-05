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
*/

#[macro_use]
extern crate lazy_static;

use tera::{Context, Tera};
use std::{
    net::{TcpListener, TcpStream},
    io::{prelude::*, BufReader}
};


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

    let status_line = "HTTP/1.1 200 OK";
    let length = contents.len();

    let response = format!("{status_line}\r\nContentLength: {length}\r\n\r\n{contents}");

    println!("Request: {:#?}", http_request);
    println!("Response: {:#?}", response);

    stream.write_all(response.as_bytes()).unwrap();
}