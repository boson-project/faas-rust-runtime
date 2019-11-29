extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{quote, format_ident, quote_spanned};
use syn::{Type, FnArg, Path, PathArguments, GenericArgument, spanned::Spanned, Ident, ExprCall};
use std::borrow::Borrow;

#[proc_macro_attribute]
pub fn faas_function(_args: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let function_ast: syn::ItemFn = syn::parse(item.clone()).unwrap();

    let user_function: TokenStream = item.into();
    let main_fn: TokenStream = quote! {
        fn main() {
            faas_rust::start_runtime(|r| r.to(handle_event))
        }
    };
    let handler = generate_handler(function_ast);

    let out = quote! {
        #user_function
        #handler
        #main_fn
    };

    out.into()
}

fn generate_handler(function_ast: syn::ItemFn) -> TokenStream {
    let user_function_name = function_ast.sig.ident;

    let extracted: Vec<(Ident, TokenStream)> = function_ast.sig.inputs
        .iter()
        .enumerate()
        .map(|(i, arg)|
            extract_type_from_fn_arg(arg)
                .and_then(|ty| {
                    let varname = format_ident!("_arg{}", i);
                    if is_event(ty) {
                        let num = i + 1;
                        Some((varname.clone(), quote_spanned! {arg.span()=>
                            let #varname = events.remove_item(0).ok_or(actix_web::error::ErrorBadRequest(format!("Expecting event in position {}", #num)));
                        }))
                    } else if is_option_event(ty) {
                        Some((varname.clone(), quote_spanned! {arg.span()=>
                            let #varname = events.remove_item(0);
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

    let (extracted_ident, extracted_stmts): (Vec<Ident>, Vec<TokenStream>) = extracted
        .iter()
        .cloned()
        .unzip();

    let mut user_function_invocation = quote! {
            #user_function_name(#(#extracted_ident),*)
    };

    if function_ast.sig.asyncness.is_some() {
        user_function_invocation = quote! {
            #user_function_invocation.await
        }
    };

    let out = quote! {
            async fn handle_event(
            req: actix_web::HttpRequest,
            body: actix_web::web::Bytes,
        ) -> Result<actix_web::HttpResponse, actix_web::Error> {
            let value = faas_rust::request_reader::read_cloud_event(req, body).await?;

            // Unzip
            let (encoding, events) = match value {
                Some((encoding, events)) => (Some(encoding), Some(events)),
                None => (None, vec![])
            };

            #(#extracted_stmts)*

            let output = #user_function_invocation?;
            faas_rust::response_writer::write_cloud_event(output, encoding)
        }
    };

    out.into()
}

fn is_option_event(ty: &Type) -> bool {
    let extracted = extract_type_from_option(ty);
    match extracted {
        Some(Type::Path(type_path)) => type_path.path.segments.last().unwrap().ident == "Event",
        _ => false
    }
}

fn is_event(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => type_path.path.segments.last().unwrap().ident == "Event",
        _ => false
    }
}

fn extract_type_from_option(ty: &Type) -> Option<&Type> {
    fn path_is_option(path: &Path) -> bool {
        path.leading_colon.is_none()
            && path.segments.len() == 1
            && path.segments.iter().next().unwrap().ident == "Option"
    }

    match ty {
        Type::Path(type_path) if type_path.qself.is_none() && path_is_option(&type_path.path) => {
            // Get the first segment of the path (there is only one, in fact: "Option"):
            let type_params = &type_path.path.segments.first().unwrap().arguments;
            // It should have only on angle-bracketed param ("<String>"):
            let generic_arg = match type_params {
                PathArguments::AngleBracketed(params) => params.args.first().unwrap(),
                _ => return None,
            };
            // This argument must be a type:
            match generic_arg {
                GenericArgument::Type(ty) => Some(ty),
                _ => return None,
            }
        }
        _ => return None,
    }
}

fn extract_type_from_fn_arg(fn_arg: &FnArg) -> Option<&Type> {
    match fn_arg {
        FnArg::Typed(ty) => Some(ty.ty.borrow()),
        _ => None
    }
}