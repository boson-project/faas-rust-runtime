extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::quote;

mod types;
mod input_parsing;
mod handler_gen;

// Allowed inputs:
// - Event
// - Option<Event>
// - N Event followed by N Option<Event>
// - HashMap<String, Event>
// - Vec<Event>

// Allowed outputs (wrapped in Result<X, actix_web::Error>):
// - Event
// - Option<Event>
// - HashMap<String, Event>
// - Vec<Event>

#[proc_macro_attribute]
pub fn faas_function(
    _args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let function_ast: syn::ItemFn = syn::parse(item.clone()).unwrap();
    let out = impl_faas_function(function_ast);
    out.into()
}

fn impl_faas_function(user_function: syn::ItemFn) -> TokenStream {
    let main_fn: TokenStream = quote! {
        #[actix_rt::main]
        async fn main() -> std::io::Result<()> {
            faas_rust::start_runtime(|r| r.to(handle_event)).await
        }
    };
    let handler = generate_handler(&user_function);

    quote! {
        use std::iter::FromIterator;

        #user_function
        #handler
        #main_fn
    }
}

fn generate_handler(function_ast: &syn::ItemFn) -> TokenStream {
    let inner_body = generate_handler_inner_body(function_ast);
    let output_extraction = handler_gen::generate_output_extraction(&function_ast.sig.output).unwrap_or(
        syn::Error::new_spanned(
            &function_ast.sig.output,
            "Return type should be Result<V, E>, where V is HashMap<String, Event> or Vec<Event> or Option<Event> or Event",
        ).to_compile_error(),
    );

    let out = quote! {
            async fn handle_event(
            req: actix_web::HttpRequest,
            body: actix_web::web::Bytes,
        ) -> Result<actix_web::HttpResponse, actix_web::Error> {
            let mut was_binary = true;
            let input = faas_rust::request_reader::read_cloud_event(req, body).await?;

            #inner_body

            let output = #output_extraction;
            faas_rust::response_writer::write_cloud_event(output)
        }
    };

    out.into()
}

fn generate_handler_inner_body(function_ast: &syn::ItemFn) -> TokenStream {
    // Function input
    let mut extracted_fn_params = input_parsing::extract_fn_params(&function_ast);

    if extracted_fn_params.is_empty() {
        return handler_gen::generate_no_input_handler_body(function_ast)
    }
    if extracted_fn_params.len() == 1 {
        let (_, ty) = extracted_fn_params.remove(0);
        if types::is_vec_event(ty) {
            // Generate Vec<Event> input case
            return handler_gen::generate_vec_handler_body(function_ast)
        } else if types::is_hashmap_event(ty) {
            // Generate HashMap<String, Event> input case
            return handler_gen::generate_hashmap_handler_body(function_ast)
        } else {
            // Generate Event or Option<Event> input case
            return handler_gen::generate_single_event_handler_body(function_ast, ty)
        }
    }
    return handler_gen::generate_multi_event_handler_body(function_ast, extracted_fn_params)
}
