use actix_web::http::HeaderMap;
use actix_web::web::Bytes;
use actix_web::HttpRequest;
use chrono::DateTime;
use cloudevent::{Event, Payload};
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
pub async fn read_cloud_event(
    req: HttpRequest,
    payload: Bytes,
) -> Result<Option<(Encoding, Vec<Event>)>, actix_web::Error> {
    let mut headers: HeaderMap = req.headers().clone();

    if let Ok(ct) = unwrap_and_remove_header!(headers, "content-type") {
        if ct.contains("application/cloudevents+json") {
            // Payload at this point should not be none
            if payload.is_empty() {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "No payload provided but content type is {}",
                    ct
                )))
            } else {
                return parse_structured(payload).await
                    .map(|ce| Some((Encoding::STRUCTURED, vec![ce])))
            }
        } else {
            if payload.is_empty() {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "No payload provided but content type is {}",
                    ct
                )))
            } else {
                return parse_binary(headers, Some((ct, payload))).await
                        .map(|ce| Some((Encoding::BINARY, vec![ce])))
            }
        }
    }

    if headers.contains_key(CE_ID_HEADER) {
        return parse_binary(headers, None).await
            .map(|ce| Some((Encoding::BINARY, vec![ce])))
    }

    return Ok(None)
}

async fn parse_structured(
    payload: Bytes,
) -> Result<Event, actix_web::Error> {
    serde_json::from_slice::<Event>(&payload)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("{}", e)))
}

async fn parse_binary(
    headers: HeaderMap,
    payload: Option<(String, Bytes)>,
) -> Result<Event, actix_web::Error> {
    if payload.is_some() {
        let (ct, p) = payload.unwrap();

        let mut ce = Event::new();
        read_ce_headers(headers, &mut ce)?;
        if !p.is_empty() {
            ce.payload = Some(Payload {
                content_type: ct,
                data: p.to_vec(),
            });
        }
        Ok(ce)
    } else {
        let mut ce = Event::new();
        read_ce_headers(headers, &mut ce)?;
        Ok(ce)
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
