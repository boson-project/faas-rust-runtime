#![feature(start)]

use cloudevent::Event;
use faas_rust_macro::faas_function;

#[faas_function]
pub async fn function(
    last: Event,
    _other: Event
) -> Result<Event, actix_web::Error> {
    Ok(last)
}

#[start]
fn start(_argc: isize, _argv: *const *const u8) -> isize {
    0
}
