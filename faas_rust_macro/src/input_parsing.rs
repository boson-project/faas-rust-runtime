use syn::{PatType, PatIdent, Type, FnArg, Ident};
use syn::visit::Visit;

pub(crate) fn extract_fn_params(function_ast: &syn::ItemFn) -> Vec<(&Ident, &Type)> {
    function_ast.sig.inputs
        .iter()
        .map(extract_type_from_fn_arg)
        .filter_map(std::convert::identity)
        .collect()
}

pub(crate) fn extract_type_from_fn_arg(fn_arg: &FnArg) -> Option<(&Ident, &Type)> {
    let mut extractor = FnArgExtractor {
        name: None,
        ty: None
    };
    extractor.visit_fn_arg(fn_arg);
    match extractor {
        FnArgExtractor{name: Some(n), ty: Some(ty)} => Some((n, ty)),
        _ => None
    }
}

struct FnArgExtractor<'a> {
    name: Option<&'a Ident>,
    ty: Option<&'a Type>,
}

impl<'ast> Visit<'ast> for FnArgExtractor<'ast> {

    fn visit_pat_ident(&mut self, i: &'ast PatIdent) {
        self.name = Some(&i.ident);
        syn::visit::visit_pat_ident(self, i)
    }

    fn visit_pat_type(&mut self, i: &'ast PatType) {
        self.ty = Some(&i.ty);
        syn::visit::visit_pat_type(self, i)
    }
}

#[cfg(tests)]
mod tests{
    use super::*;

    #[test]
    fn extract_params_from_item_fn() -> Result<(), syn::Error> {
        let function_ast: syn::ItemFn = syn::parse_str(r"fn function(first: Option<Event>, other: Vec<Event>) -> Result<Event, actix_web::Error> {}")?;
        let mut params = extract_fn_params(&function_ast);
        let (param1_name, param1_type) = params.remove(0);
        assert_eq!(param1_name.to_string(), "first");
        assert_eq!(param1_type, &syn::parse_str::<syn::Type>("Option<Event>")?);
        let (param2_name, param2_type) = params.remove(0);
        assert_eq!(param2_name.to_string(), "other");
        assert_eq!(param2_type, &syn::parse_str::<syn::Type>("Vec<Event>")?);
        Ok(())
    }
}
