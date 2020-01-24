#![feature(start)]

use cloudevent::Event;
use faas_rust_macro::faas_function;

#[faas_function]
pub async fn function(
    _last: Vec<Event>
) -> Result<Option<Event>, actix_web::Error> {
    Ok(None)
}

#[start]
fn start(_argc: isize, _argv: *const *const u8) -> isize {
    0
}
