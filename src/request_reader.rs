use actix_web::http::HeaderMap;
use actix_web::web::BytesMut;
use actix_web::{web, HttpRequest};
use chrono::DateTime;
use cloudevent::{Event, Payload};
use futures::{Future, IntoFuture, Stream};
use std::convert::TryInto;
use cloudevent::http::*;

macro_rules! unwrap_header {
    ($headers:expr, $key:expr) => {
        $headers
            .get($key)
            .ok_or(actix_web::error::ErrorBadRequest(format!(
                "Expecting header {}",
                $key
            )))
            .and_then(|ce| {
                ce.to_str().map(|s| String::from(s)).map_err(|e| {
                    actix_web::error::ErrorBadRequest(format!(
                        "Error while parsing header {}: {}",
                        $key, e
                    ))
                })
            })
    };
}

macro_rules! unwrap_and_remove_header {
    ($headers:expr, $key:expr) => {{
        let v = unwrap_header!($headers, $key);
        $headers.remove($key);
        v
    }};
}

// Possible cases:
// 1. Content-type exists:
// 1.1 If application/cloudevents+json -> parse structured
// 1.2 If other -> parse binary
// 2. Content-type doesn't exist:
// 2.1 If CE id header, then it's an empty payload cloud event -> parse binary
// 2.2 If no CE header -> None
pub fn read_cloud_event(
    req: HttpRequest,
    payload: Option<web::Payload>,
) -> Box<dyn Future<Item = Option<(Encoding, Event)>, Error = actix_web::Error>> {
    let mut headers: HeaderMap = req.headers().clone();

    if let Ok(ct) = unwrap_and_remove_header!(headers, "content-type") {
        if ct.contains("application/cloudevents+json") {
            // Payload at this point should not be none
            if payload.is_none() {
                return Box::new(futures::failed(actix_web::error::ErrorBadRequest(format!(
                    "No payload provided but content type is {}",
                    ct
                ))));
            } else {
                return Box::new(
                    parse_structured(payload.unwrap())
                        .map(|ce| Some((Encoding::STRUCTURED, ce))),
                );
            }
        } else {
            if payload.is_none() {
                return Box::new(futures::failed(actix_web::error::ErrorBadRequest(format!(
                    "No payload provided but content type is {}",
                    ct
                ))));
            } else {
                return Box::new(
                    parse_binary(headers, Some((ct, payload.unwrap())))
                        .map(|ce| Some((Encoding::BINARY, ce))),
                );
            }
        }
    }

    if headers.contains_key(CE_ID_HEADER) {
        return Box::new(
            parse_binary(headers, None).map(|ce| Some((Encoding::BINARY, ce))),
        );
    }

    return Box::new(futures::finished(None));
}

fn parse_structured(
    payload: web::Payload,
) -> impl Future<Item =Event, Error = actix_web::Error> {
    read_body(payload).and_then(|b| {
        serde_json::from_slice::<Event>(&b)
            .into_future()
            .map_err(|e| actix_web::error::ErrorBadRequest(format!("{}", e)))
    })
}

fn parse_binary(
    headers: HeaderMap,
    payload: Option<(String, web::Payload)>,
) -> Box<dyn Future<Item =Event, Error = actix_web::Error>> {
    if payload.is_some() {
        let (ct, b) = payload.unwrap();

        Box::new(
            read_body(b)
                .and_then(|b| {
                    let mut ce = Event::new();
                    read_ce_headers(headers, &mut ce)?;
                    let body = std::str::from_utf8(&b);
                    if body.is_ok() {
                        ce.payload = Some(Payload {
                            content_type: ct,
                            data: String::from(body.unwrap()),
                        });
                        return Ok(ce);
                    } else {
                        return Err(actix_web::error::ErrorBadRequest(format!(
                            "Cannot decode body: {}",
                            body.err().unwrap()
                        )));
                    }
                })
                .into_future(),
        )
    } else {
        let mut ce = Event::new();
        Box::new(futures::done(
            read_ce_headers(headers, &mut ce).map(|_| ce)
        ))
    }
}

fn read_ce_headers(mut headers: HeaderMap, ce: &mut Event) -> Result<(), actix_web::Error> {
    if headers.contains_key(CE_ID_HEADER) {
        ce.id = unwrap_and_remove_header!(headers, CE_ID_HEADER)?;
        ce.event_type = unwrap_and_remove_header!(headers, CE_TYPE_HEADER)?;
        ce.spec_version = unwrap_and_remove_header!(headers, CE_SPECVERSION_HEADER).and_then(|sv| {
            sv.try_into()
                .map_err(|e| actix_web::error::ErrorBadRequest(format!("{}", e)))
        })?;
        ce.source = unwrap_and_remove_header!(headers, CE_SOURCE_HEADER)?;
        ce.subject = unwrap_and_remove_header!(headers, CE_SUBJECT_HEADER).ok();
        ce.time = unwrap_and_remove_header!(headers, CE_TIME_HEADER)
            .and_then(|t| {
                DateTime::parse_from_rfc3339(&t)
                    .map_err(|e| actix_web::error::ErrorBadRequest(format!("{}", e)))
            })
            .ok();

        //TODO extensions
    }

    Ok(())
}

fn read_body(body: web::Payload) -> impl Future<Item = BytesMut, Error = actix_web::Error> {
    body.map_err(actix_web::Error::from)
        .fold(BytesMut::new(), move |mut body, chunk| {
            body.extend_from_slice(&chunk);
            Ok::<_, actix_web::Error>(body)
        })
}
