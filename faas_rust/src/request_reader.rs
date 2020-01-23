use actix_web::http::{HeaderMap, header};
use actix_web::web::Bytes;
use actix_web::HttpRequest;
use chrono::DateTime;
use cloudevent::http::*;
use cloudevent::{Event, Payload};
use std::convert::TryInto;
use std::collections::HashMap;
use crate::common::EventRequest;

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
// 1.2 If application/cloudevents-batch+json -> parse batch
// 1.3 If application/cloudevents-bundle+json -> parse bundle
// 1.4 If other -> parse binary
// 2. Content-type doesn't exist:
// 2.1 If CE id header, then it's an empty payload cloud event -> parse binary
// 2.2 If no CE header -> None
pub async fn read_cloud_event(
    req: HttpRequest,
    payload: Bytes,
) -> Result<EventRequest, actix_web::Error> {
    let mut headers: HeaderMap = req.headers().clone();

    if let Ok(ct) = unwrap_and_remove_header!(headers, "content-type") {
        // Payload at this point can't be None
        if payload.is_empty() {
            return Err(actix_web::error::ErrorBadRequest(format!(
                "No payload provided but content type is {}",
                ct
            )));
        }
        if ct.contains(CE_JSON_CONTENT_TYPE) {
            return parse_structured(payload)
                .await
                .map(|ce| EventRequest::Structured(Some(ce)));
        } else if ct.contains(CE_BUNDLE_JSON_CONTENT_TYPE) {
            return parse_bundle(payload)
                .await
                .map(EventRequest::Bundle);
        } else if ct.contains(CE_BATCH_JSON_CONTENT_TYPE) {
            return parse_batch(payload)
                .await
                .map(EventRequest::Batch);
        } else {
            return parse_binary(headers, Some((ct, payload)))
                .await
                .map(|ce| EventRequest::Binary(Some(ce)));
        }
    }

    if headers.contains_key(CE_ID_HEADER) {
        return parse_binary(headers, None)
            .await
            .map(|ce| EventRequest::Binary(Some(ce)));
    }

    return Ok(EventRequest::Binary(None));
}

async fn parse_structured(payload: Bytes) -> Result<Event, actix_web::Error> {
    serde_json::from_slice::<Event>(&payload)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("{}", e)))
}

async fn parse_batch(payload: Bytes) -> Result<Vec<Event>, actix_web::Error> {
    serde_json::from_slice::<Vec<Event>>(&payload)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("{}", e)))
}

async fn parse_bundle(payload: Bytes) -> Result<HashMap<String, Event>, actix_web::Error> {
    serde_json::from_slice::<HashMap<String, Event>>(&payload)
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
        ce.spec_version =
            unwrap_and_remove_header!(headers, CE_SPECVERSION_HEADER).and_then(|sv| {
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

        let extensions = headers
            .iter()
            .map(|(name, value)| (name.as_str(), value))
            .filter(|(name, _)| name.starts_with("ce-"))
            .map(|(name, value)| Ok((name.to_string(), value.to_str()?.to_string())))
            .collect::<Result<Vec<(String, String)>, header::ToStrError>>()
            .map_err(|e| actix_web::error::ErrorBadRequest(format!("{}", e)))?;

        ce.extensions = extensions.into_iter().collect();
    }

    Ok(())
}
