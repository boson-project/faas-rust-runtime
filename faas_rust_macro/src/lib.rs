extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn faas_function(_args: TokenStream, item: TokenStream) -> TokenStream {
    let ast: syn::ItemFn = syn::parse(item.clone()).unwrap();
    let item2: proc_macro2::TokenStream = item.into();

    // TODO check arguments and return type!

    // Build the main function
    let name = ast.sig.ident;
    let gen = quote! {
        #item2

        async fn handle_event(
            req: actix_web::HttpRequest,
            body: actix_web::web::Bytes,
        ) -> Result<actix_web::HttpResponse, actix_web::Error> {
            let value = faas_rust::request_reader::read_cloud_event(req, body).await?;

            // Unzip
            let (encoding, event) = match value {
                Some((encoding, event)) => (Some(encoding), Some(event)),
                None => (None, None)
            };

            let output = #name(event).await?;
            faas_rust::response_writer::write_cloud_event(output, encoding)
        }

        fn main() {
            faas_rust::start_runtime(|r| r.to(handle_event))
        }
    };
    gen.into()
}