use cloudevent::Event;
use maplit::hashmap;
use faas_rust_macro::faas_function;

#[faas_function]
pub async fn function() -> Result<HashMap<String, Event>, actix_web::Error> {
    Ok(hashmap!{})
}
