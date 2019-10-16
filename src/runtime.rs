extern crate serde_json;
extern crate futures;

use self::futures::{IntoFuture, Future, Stream};
use actix_web::{App, HttpServer, web, HttpResponse, Responder, HttpRequest, FromRequest};
use actix_web::web::BytesMut;
use std::net::SocketAddr;
use std::env;
use actix_web::http::Method;

type UserFunction = fn(Option<serde_json::Value>) -> Box<dyn futures::Future<Item=Option<serde_json::Value>, Error=actix_web::Error>>;

const PORT_ENV: &str = "ENV";

#[derive(Copy, Clone)]
struct Runtime {
    user_function: UserFunction
}

impl Runtime {
    fn handle_get_event<'a, 'b>(&'a self) -> Box<(dyn Future<Item=HttpResponse, Error=actix_web::Error> + 'b)>
        where 'a: 'b
    {
        self.invoke_function::<'a, 'b>(None)
    }

    fn handle_post_event<'a, 'b>(&'a self, req: &mut HttpRequest) -> Box<(dyn Future<Item=HttpResponse, Error=actix_web::Error> + 'b)>
        where 'a: 'b
    {
        Box::new(
            futures::done(web::Payload::extract(req)).and_then(|p|
                p
                    .map_err(actix_web::Error::from)
                    .fold(BytesMut::new(), move |mut body, chunk| {
                        body.extend_from_slice(&chunk);
                        Ok::<_, actix_web::Error>(body)
                    }).and_then(|body|
                    serde_json::from_slice::<serde_json::Value>(&body)
                        .into_future()
                        .map_err(|e| actix_web::error::ErrorBadRequest(format!("{}", e)))
                )
                    .map(Option::Some)
                    .and_then(|j| self.invoke_function(j))
            )
        )
    }

    fn invoke_function<'a, 'b>(&'a self, value: Option<serde_json::Value>) -> Box<(dyn Future<Item=HttpResponse, Error=actix_web::Error> + 'b)>
        where 'a: 'b // Nothing fancy, just saying that self lives at least as long as future result
    {
        let fn_result = (self.user_function)(value);
        Box::new(
            fn_result
                .map(|val|
                    val.map_or_else(
                        || HttpResponse::Accepted().finish(),
                        |v| HttpResponse::Ok().content_type("application/json").body(format!("{}", v)),
                    )
                )
                .map_err(|err| actix_web::error::ErrorInternalServerError(format!("{}", err)))
        )
    }
}

impl Responder for Runtime {
    type Error = actix_web::Error;
    type Future = Box<dyn futures::Future<Item=HttpResponse, Error=Self::Error>>;

    fn respond_to(self, req: &HttpRequest) -> Self::Future {
        match req.method() {
            &Method::GET => self.handle_get_event(),
            &Method::POST | &Method::PUT => self.handle_post_event(&mut req.clone())
        }
    }
}

pub fn start_runtime(user_function: UserFunction) {
    let port: u16 = env::var(PORT_ENV)
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8080);

    let addr: SocketAddr = ([127, 0, 0, 1], port).into();

    HttpServer::new(|| {
        let runtime = Runtime { user_function };
        App::new()
            .route("/", web::get().to_async(move || runtime))
            .route("/", web::post().to_async(move || runtime))
    })
        .bind(addr)
        .expect("Cannot bind to port 8080")
        .run()
        .unwrap();
}

