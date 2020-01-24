use proc_macro2::TokenStream;
use syn::{Ident, ItemFn, Type, ReturnType};
use syn::spanned::Spanned;
use quote::{format_ident, quote, quote_spanned};
use super::types::*;

pub(crate) fn generate_vec_handler_body(function_ast: &ItemFn) -> TokenStream {
    let arg_ident = Ident::new("_arg0", function_ast.span());
    let user_function_invocation = generate_user_function_invocation(function_ast, vec![arg_ident]);
    quote_spanned! {function_ast.span()=>
        let _arg0: Vec<Event> = match input {
            faas_rust::common::EventRequest::Binary(opt) => Ok(opt.into_iter().collect()),
            faas_rust::common::EventRequest::Structured(opt) => {
                was_binary = false;
                Ok(opt.into_iter().collect())
            },
            faas_rust::common::EventRequest::Batch(v) => Ok(v),
            faas_rust::common::EventRequest::Bundle(_) => Err(actix_web::error::ErrorBadRequest(format!("This function doesn't accept cloudevent bundle")))
        }?;
        let function_output = #user_function_invocation?;
    }
}

pub(crate) fn generate_hashmap_handler_body(function_ast: &ItemFn) -> TokenStream {
    let arg_ident = Ident::new("_arg0", function_ast.span());
    let user_function_invocation = generate_user_function_invocation(function_ast, vec![arg_ident]);
    quote_spanned! {function_ast.span()=>
        let _arg0: std::collections::HashMap<String, Event> = match input {
            faas_rust::common::EventRequest::Binary(_) => Err(actix_web::error::ErrorBadRequest(format!("This function doesn't accept cloudevent binary"))),
            faas_rust::common::EventRequest::Structured(_) => Err(actix_web::error::ErrorBadRequest(format!("This function doesn't accept cloudevent structured"))),
            faas_rust::common::EventRequest::Batch(_) => Err(actix_web::error::ErrorBadRequest(format!("This function doesn't accept cloudevent batch"))),
            faas_rust::common::EventRequest::Bundle(m) => Ok(m)
        }?;
        let function_output = #user_function_invocation?;
    }
}

pub(crate) fn generate_single_event_handler_body(function_ast: &ItemFn, param_type: &Type) -> TokenStream {
    let arg_ident = Ident::new("_arg0", function_ast.span());
    let user_function_invocation = generate_user_function_invocation(function_ast, vec![arg_ident]);

    let opt_unwrapped = if is_option_event(param_type) {
        quote!{Ok(opt)}
    } else {
        quote!{opt.ok_or(actix_web::error::ErrorBadRequest(format!("Expecting a non empty Event")))}
    };

    quote_spanned! {function_ast.span()=>
        let _arg0: #param_type = match input {
            faas_rust::common::EventRequest::Binary(opt) => #opt_unwrapped,
            faas_rust::common::EventRequest::Structured(opt) => {
                was_binary = false;
                #opt_unwrapped
            },
            faas_rust::common::EventRequest::Batch(_) => Err(actix_web::error::ErrorBadRequest(format!("This function doesn't accept cloudevent batch"))),
            faas_rust::common::EventRequest::Bundle(_) => Err(actix_web::error::ErrorBadRequest(format!("This function doesn't accept cloudevent bundle")))
        }?;
        let function_output = #user_function_invocation?;
    }
}

pub(crate) fn generate_no_input_handler_body(function_ast: &ItemFn) -> TokenStream {
    let user_function_invocation = generate_user_function_invocation(function_ast, vec![]);

    quote_spanned! {function_ast.span()=>
        let function_output = #user_function_invocation?;
    }
}

pub(crate) fn generate_multi_event_handler_body(function_ast: &ItemFn, params: Vec<(&Ident, &Type)>) -> TokenStream {
    let extraction_stmts: Vec<(Ident, TokenStream)> = params
        .into_iter()
        .enumerate()
        .map(|(i, (ident, ty))| {
            let var_name = format_ident!("_arg{}", i);
            let key_literal = syn::LitStr::new(ident.to_string().as_str(), ident.span());
            if is_event(ty) {
                (var_name.clone(), quote_spanned! {ident.span()=>
                    let #var_name: cloudevent::Event = input_map.remove(#key_literal).ok_or(actix_web::error::ErrorBadRequest(format!("Cannot find event with name {}", #key_literal)))?;
                })
            } else if is_option_event(ty) {
                (var_name.clone(), quote_spanned! {ident.span()=>
                    let #var_name: Option<cloudevent::Event> = input_map.remove(#key_literal);
                })
            } else {
                let err = syn::Error::new_spanned(ident.clone(), "Type should be Event or Option<Event>").to_compile_error();
                (
                    var_name.clone(),
                    quote_spanned! {ident.span()=>
                        let #var_name: Option<cloudevent::Event> = #err;
                    }
                )
            }
        })
        .collect();

    let (input_extracted_ident, input_extracted_stmts): (Vec<Ident>, Vec<TokenStream>) =
        extraction_stmts.iter().cloned().unzip();

    let user_function_invocation = generate_user_function_invocation(function_ast, input_extracted_ident);

    quote_spanned! {function_ast.span()=>
        let mut input_map: std::collections::HashMap<String, Event> = match input {
            faas_rust::common::EventRequest::Binary(_) => Err(actix_web::error::ErrorBadRequest(format!("This function doesn't accept cloudevent binary"))),
            faas_rust::common::EventRequest::Structured(_) => Err(actix_web::error::ErrorBadRequest(format!("This function doesn't accept cloudevent structured"))),
            faas_rust::common::EventRequest::Batch(_) => Err(actix_web::error::ErrorBadRequest(format!("This function doesn't accept cloudevent batch"))),
            faas_rust::common::EventRequest::Bundle(m) => Ok(m)
        }?;
        #(#input_extracted_stmts)*
        let function_output = #user_function_invocation?;
    }
}

// pub enum EventRequest {
//    Binary(Option<Event>),
//    Structured(Option<Event>),
//    Batch(Vec<Event>),
//    Bundle(HashMap<String, Event>)
//}

fn generate_user_function_invocation(function_ast: &ItemFn, params: Vec<Ident>) -> TokenStream {
    let user_function_name = function_ast.sig.ident.clone();

    let mut user_function_invocation = quote! {
            #user_function_name(#(#params),*)
    };

    if function_ast.sig.asyncness.is_some() {
        user_function_invocation = quote! {
            #user_function_invocation.await
        }
    };

    return user_function_invocation
}

pub(crate) fn generate_output_extraction(rt: &ReturnType) -> Option<TokenStream> {
    let result_type = match rt {
        ReturnType::Type(_, ty) => extract_types_from_result(&ty),
        _ => None,
    }?;
    let result_left = result_type.0;

    if is_hashmap_event(result_left) {
        Some(quote! {
        faas_rust::common::EventResponse::Bundle(function_output)
        })
    } else if is_vec_event(result_left) {
        Some(quote! {
        faas_rust::common::EventResponse::Batch(function_output)
        })
    } else if is_option_event(result_left) {
        Some(quote! {
        if was_binary {
            faas_rust::common::EventResponse::Binary(function_output)
        } else {
            faas_rust::common::EventResponse::Structured(function_output)
        }
        })
    } else if is_event(result_left) {
        Some(quote! {
        if was_binary {
            faas_rust::common::EventResponse::Binary(Some(function_output))
        } else {
            faas_rust::common::EventResponse::Structured(Some(function_output))
        }
        })
    } else {
        None
    }
}
