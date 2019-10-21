use rust_faas::start_runtime;

#[cfg(not(feature = "external-function"))]
mod function;

#[cfg(feature = "external-function")]
extern crate function;
#[cfg(feature = "external-function")]
use function;

fn main() {
    start_runtime(function::function)
}
