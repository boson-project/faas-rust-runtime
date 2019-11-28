extern crate futures;
extern crate serde_json;

mod request_reader;
mod response_writer;

use actix_web::{web, App, HttpResponse, HttpServer, HttpRequest};
use cloudevent::Event;
use cloudevent::http::Encoding;
use futures::{Future, IntoFuture};
use std::env;
use std::net::SocketAddr;

const PORT_ENV: &str = "ENV";

type UserFunction = fn(
    Option<Event>,
) -> Box<
    dyn futures::Future<Item = Option<Event>, Error = actix_web::Error>,
>;

fn invoke_function(
    user_function: web::Data<UserFunction>,
    value: Option<(Encoding, Event)>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    // Unzip
    let (encoding, event) = match value {
        Some((encoding, event)) => (Some(encoding), Some(event)),
        None => (None, None)
    };

    user_function.get_ref()(event)
        .and_then(|res| response_writer::write_cloud_event(res, encoding).into_future())
}

fn handle_get_event(
    req: HttpRequest,
    user_function: web::Data<UserFunction>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    return request_reader::read_cloud_event(req, None)
        .and_then(|r| invoke_function(user_function, r))
}

fn handle_post_event(
    user_function: web::Data<UserFunction>,
    req: HttpRequest,
    body: web::Payload,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    return request_reader::read_cloud_event(req, Some(body))
        .and_then(|r| invoke_function(user_function, r))
}

pub fn start_runtime(user_function: UserFunction) {
    let port: u16 = env::var(PORT_ENV)
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8080);

    let addr: SocketAddr = ([127, 0, 0, 1], port).into();

    HttpServer::new(move || {
        App::new()
            .data(user_function)
            .route("/", web::get().to_async(handle_get_event))
            .route("/", web::post().to_async(handle_post_event))
    })
    .bind(addr)
    .expect("Cannot bind to port 8080")
    .run()
    .unwrap();
}