use rust_faas::start_runtime;

#[cfg(not(feature = "external-function"))]
mod function;

#[cfg(feature = "external-function")]
extern crate function;

fn main() {
    start_runtime(function::function)
}
