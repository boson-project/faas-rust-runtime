use cloudevent::Event;
use faas_rust_macro::faas_function;

#[faas_function]
pub async fn function(
    last: Event
) -> Result<Event, actix_web::Error> {
    Ok(last)
}
