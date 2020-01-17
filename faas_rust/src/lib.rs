extern crate futures;
extern crate serde_json;

pub mod request_reader;
pub mod response_writer;

use actix_web::{guard, Route};
use std::env;
use std::net::SocketAddr;

const PORT_ENV: &str = "PORT";
const UNIX_DOMAIN_SOCKET_ENV: &str = "UNIX_DOMAIN_SOCKET";
const LOG_ENV: &str = "FAAS_LOG";

fn configure_logging() {
    let enable: bool = env::var(LOG_ENV)
        .ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);
    if enable {
        ::std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
        env_logger::init();
    }
}

fn get_bind_address() -> SocketAddr {
    let port: u16 = env::var(PORT_ENV)
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8080);

    ([0, 0, 0, 0], port).into()
}

pub async fn start_runtime(route_mod_fn: fn(Route) -> Route) -> std::io::Result<()> {
    configure_logging();

    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .wrap(actix_web::middleware::Logger::default())
            .route(
                "/*",
                route_mod_fn(
                    actix_web::web::route().guard(guard::Any(guard::Get()).or(guard::Post())),
                ),
            )
    });

    if let Some(uds_address) = env::var(UNIX_DOMAIN_SOCKET_ENV).ok() {
        println!(
            "FaaS Runtime: Starting server listening Unix Domain Socket {}",
            uds_address
        );
        server
            .bind_uds(&uds_address)
            .expect(format!("Cannot bind uds {}", uds_address).as_ref())
            .run()
            .await
    } else {
        let addr: std::net::SocketAddr = get_bind_address();
        println!("Starting server listening {}", addr);
        server
            .bind(addr)
            .expect(format!("Cannot bind address {}", addr).as_ref())
            .run()
            .await
    }
}
