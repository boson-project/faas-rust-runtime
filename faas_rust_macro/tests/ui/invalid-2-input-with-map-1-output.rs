use cloudevent::Event;
use faas_rust_macro::faas_function;

#[faas_function]
pub async fn function(
    last: std::collections::HashMap<Event>,
    other: Event,
) -> Result<Event, actix_web::Error> {
    Ok(other)
}
