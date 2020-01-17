use cloudevent::{Event, Reader, Writer};
use serde_json::json;

#[faas_rust_macro::faas_function]
pub async fn fold(
    last: Event,
    aggregator: Option<Event>,
) -> Result<Option<Event>, actix_web::Error> {
    println!("Received {:?}", last);
    let input_json = last
        .read_payload()
        .and_then(|e| e.ok())
        .unwrap_or(serde_json::Value::Null);

    println!("Input json: {}", input_json);
    let name = input_json
        .as_object()
        .and_then(|o| o.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("World");

    let json = json!({ "Hello": name });
    let mut result_ce = last.clone();
    let _ = result_ce.write_payload("application/json", json);

    Ok(Some(result_ce))
}
