extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use std::borrow::Borrow;
use syn::{spanned::Spanned, FnArg, Ident, ReturnType, Type};

mod types;

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
    let handler = generate_handler(user_function.clone());

    quote! {
        use std::iter::FromIterator;

        #user_function
        #handler
        #main_fn
    }
}

fn generate_handler(function_ast: syn::ItemFn) -> TokenStream {
    let user_function_name = function_ast.sig.ident.clone();

    // Function input

    let input_extracted: Vec<(Ident, TokenStream)> = function_ast.sig.inputs
        .iter()
        .enumerate()
        .map(|(i, arg)|
            extract_type_from_fn_arg(arg)
                .and_then(|ty| {
                    let varname = format_ident!("_arg{}", i);
                    if is_event(ty) {
                        let num = i + 1;
                        Some((varname.clone(), quote_spanned! {arg.span()=>
                            let #varname: cloudevent::Event = events.pop().ok_or(actix_web::error::ErrorBadRequest(format!("Expecting event in position {}", #num)))?;
                        }))
                    } else if is_option_event(ty) {
                        Some((varname.clone(), quote_spanned! {arg.span()=>
                            let #varname: Option<cloudevent::Event> = events.pop();
                        }))
                    } else {
                        None
                    }
                })
                .unwrap_or((
                    format_ident!("{}", "err"),
                    syn::Error::new_spanned(arg, "Type should be Event or Option<Event>").to_compile_error()
                ))

        )
        .collect();

    let (input_extracted_ident, input_extracted_stmts): (Vec<Ident>, Vec<TokenStream>) =
        input_extracted.iter().cloned().unzip();

    // Function invocation

    let mut user_function_invocation = quote! {
            #user_function_name(#(#input_extracted_ident),*)
    };

    if function_ast.sig.asyncness.is_some() {
        user_function_invocation = quote! {
            #user_function_invocation.await
        }
    };

    // Function output

    let output_mapper: TokenStream = map_output(&function_ast.sig.output).unwrap_or(
        syn::Error::new_spanned(
            function_ast.sig,
            "Return type should be Result<V, E>, where V is Vec<Event> or Option<Event> or Event",
        )
        .to_compile_error(),
    );

    // fn handleEvent()

    let out = quote! {
            async fn handle_event(
            req: actix_web::HttpRequest,
            body: actix_web::web::Bytes,
        ) -> Result<actix_web::HttpResponse, actix_web::Error> {
            let value = faas_rust::request_reader::read_cloud_event(req, body).await?;

            // Unzip
            let (encoding, mut events) = match value {
                Some((encoding, events)) => (Some(encoding), events),
                None => (None, vec![])
            };

            events.reverse();

            #(#input_extracted_stmts)*

            let output = #user_function_invocation?;
            let mapped_output: Vec<cloudevent::Event> = #output_mapper;
            faas_rust::response_writer::write_cloud_event(mapped_output, encoding)
        }
    };

    out.into()
}

fn map_output(rt: &ReturnType) -> Option<TokenStream> {
    let result_type = match rt {
        ReturnType::Type(_, ty) => types::extract_types_from_result(&ty),
        _ => None,
    }?;
    let result_left = result_type.0;

    if is_vec_event(result_left) {
        Some(quote! {
        output
        })
    } else if is_option_event(result_left) {
        Some(quote! {
        Vec::from_iter(output.into_iter());
        })
    } else if is_event(result_left) {
        Some(quote! {
        vec![output]
        })
    } else {
        None
    }
}

fn is_vec_event(ty: &Type) -> bool {
    let extracted = types::extract_types_from_vec(ty);
    let type_matcher = types::generate_type_matcher("cloudevent::Event");
    match extracted {
        Some(t) => type_matcher(t),
        _ => false,
    }
}

fn is_option_event(ty: &Type) -> bool {
    let extracted = types::extract_types_from_option(ty);
    let type_matcher = types::generate_type_matcher("cloudevent::Event");
    match extracted {
        Some(t) => type_matcher(t),
        _ => false,
    }
}

fn is_event(ty: &Type) -> bool {
    types::generate_type_matcher("cloudevent::Event")(ty)
}

fn extract_type_from_fn_arg(fn_arg: &FnArg) -> Option<&Type> {
    match fn_arg {
        FnArg::Typed(ty) => Some(ty.ty.borrow()),
        _ => None,
    }
}
