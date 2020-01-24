#![feature(start)]

use cloudevent::Event;
use faas_rust_macro::faas_function;
use std::collections::HashMap;

#[faas_function]
pub async fn function() -> Result<HashMap<String, Event>, actix_web::Error> {
    Ok(std::collections::HashMap::new())
}

#[start]
fn start(_argc: isize, _argv: *const *const u8) -> isize {
    0
}
