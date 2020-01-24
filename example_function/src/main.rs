use cloudevent::{Event, Reader, Writer};
use serde_json::json;
use faas_rust_macro::faas_function;

#[faas_function]
pub async fn function(event: Event, b: Option<Event>, c: Event) -> Result<std::collections::HashMap<String, Event>, actix_web::Error> {
    Ok(std::collections::HashMap::new())
}
