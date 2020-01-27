use cloudevent::{Event, Reader, Writer};
use serde_json::json;
use faas_rust_macro::faas_function;

#[faas_function]
pub async fn function(
    last: Option<Event>
) -> Result<Event, actix_web::Error> {
    Ok(last.unwrap())
}
