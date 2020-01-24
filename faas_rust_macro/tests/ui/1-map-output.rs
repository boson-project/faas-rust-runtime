use cloudevent::Event;
use faas_rust_macro::faas_function;

#[faas_function]
pub async fn function() -> Result<HashMap<String, Event>, actix_web::Error> {
    Ok(std::collections::HashMap::new())
}
