use cloudevent::{Event, Reader, Writer};
use serde_json::json;

pub fn function(
    event: Option<Event>,
) -> Box<dyn futures::Future<Item = Option<Event>, Error = actix_web::Error>> {
    println!("Received {:?}", event);

    let input_json = event
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
    let mut result_ce = event.map(|e| e.clone()).unwrap_or(Event::new_V03());
    let _ = result_ce.write_payload("application/json", json);

    Box::new(futures::finished(Some(result_ce)))
}
