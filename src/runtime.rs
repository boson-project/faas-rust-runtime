extern crate serde_json;
extern crate futures;

use self::futures::{IntoFuture, Future, Stream};
use actix_web::{App, HttpServer, web, HttpResponse};
use actix_web::web::BytesMut;
use std::net::SocketAddr;
use std::env;
use actix_web::http::Method;

type UserFunction = fn(Option<serde_json::Value>) -> Box<dyn futures::Future<Item=Option<serde_json::Value>, Error=actix_web::Error>>;

fn invoke_function(user_function: web::Data<UserFunction>, value: Option<serde_json::Value>) -> impl Future<Item=HttpResponse, Error=actix_web::Error> {
    user_function.get_ref()(value)
        .map(|val|
            val.map_or_else(
                || HttpResponse::Accepted().finish(),
                |v| HttpResponse::Ok().content_type("application/json").body(format!("{}", v))
            )
        )
        .map_err(|err| actix_web::error::ErrorInternalServerError(format!("{}", err)))
}

fn handle_get_event(user_function: web::Data<UserFunction>) -> Box<dyn Future<Item=HttpResponse, Error=actix_web::Error>> {
    return Box::new(
        invoke_function(user_function, None)
    )
}

fn handle_post_event(user_function: web::Data<UserFunction>, body: web::Payload) -> Box<dyn Future<Item=HttpResponse, Error=actix_web::Error>> {
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
            invoke_function(user_function, Some(json))
        })
    )
}

const PORT_ENV: &str = "ENV";

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
