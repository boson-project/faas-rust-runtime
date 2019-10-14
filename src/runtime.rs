extern crate serde_json;
extern crate futures;
extern crate function;

use function::function;
use self::futures::{IntoFuture, Future, Stream};
use actix_web::{App, HttpServer, web, HttpResponse};
use actix_web::web::BytesMut;
use std::net::SocketAddr;
use std::env;

fn invoke_function(value: Option<serde_json::Value>) -> impl Future<Item=HttpResponse, Error=actix_web::Error> {
    function(value)
        .map(|val|
            val.map_or_else(
                || HttpResponse::Accepted().finish(),
                |v| HttpResponse::Ok().content_type("application/json").body(format!("{}", v))
            )
        )
        .map_err(|err| actix_web::error::ErrorInternalServerError(format!("{}", err)))
}

fn handle_get_event() -> Box<dyn Future<Item=HttpResponse, Error=actix_web::Error>> {
    return Box::new(
        invoke_function(None)
    )
}

fn handle_post_event(body: web::Payload) -> Box<dyn Future<Item=HttpResponse, Error=actix_web::Error>> {
    Box::new(
        body
            .map_err(actix_web::Error::from)
            .fold(BytesMut::new(), move |mut body, chunk| {
                body.extend_from_slice(&chunk);
                Ok::<_, actix_web::Error>(body)
            }).and_then(|body| {
            serde_json::from_slice::<serde_json::Value>(&body)
                .into_future()
                .map_err(|e| actix_web::error::ErrorBadRequest(format!("{}", e)))
        }).and_then(|json| {
            invoke_function(Some(json))
        })
    )
}

const PORT_ENV: &str = "ENV";

pub fn start_runtime() {
    let port: usize = env::var(PORT_ENV)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(8080);

    let addr: SocketAddr = ([127, 0, 0, 1], port).into();

    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to_async(handle_get_event))
            .route("/", web::post().to_async(handle_post_event))
    })
        .bind(addr)
        .expect("Cannot bind to port 8080")
        .run()
        .unwrap();
}