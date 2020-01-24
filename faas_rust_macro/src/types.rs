use syn::{GenericArgument, Path, PathArguments, Type};

pub(crate) fn extract_generics(ty: &Type, path_matcher: impl Fn(&Path) -> bool) -> Vec<&Type> {
    match ty {
        Type::Path(type_path) if type_path.qself.is_none() && path_matcher(&type_path.path) => {
            let type_params = &type_path.path.segments.last().unwrap().arguments;
            return match type_params {
                PathArguments::AngleBracketed(params) => params
                    .args
                    .iter()
                    .map(|ge| match ge {
                        GenericArgument::Type(ty) => Some(ty),
                        _ => return None,
                    })
                    .filter_map(|ty| ty)
                    .collect(),
                _ => return vec![],
            };
        }
        _ => return vec![],
    }
}

pub(crate) fn generate_path_matcher(expected_path: &str) -> impl Fn(&Path) -> bool {
    let re = regex::Regex::new(r":{2}").unwrap();
    let mut fragments: Vec<String> = re.split(expected_path).map(|v| v.to_string()).collect();
    fragments.reverse();
    move |path: &Path| {
        let path_segs: Vec<String> = path
            .segments
            .iter()
            .rev()
            .map(|s| s.ident.to_string())
            .collect();

        fragments.starts_with(&path_segs[..])
    }
}

pub(crate) fn generate_type_matcher(expected_path: &str) -> impl Fn(&Type) -> bool {
    let path_matcher = generate_path_matcher(expected_path);
    move |ty| {
        match ty {
            Type::Path(type_path) => path_matcher(&type_path.path),
            _ => false
        }
    }
}

pub(crate) fn extract_types_from_result(ty: &Type) -> Option<(&Type, &Type)> {
    let path_matcher = generate_path_matcher("std::result::Result");
    let generics = extract_generics(ty, path_matcher);
    match &generics[..] {
        &[first, second] => Some((first, second)),
        _ => None,
    }
}

pub(crate) fn extract_types_from_hashmap(ty: &Type) -> Option<(&Type, &Type)> {
    let path_matcher = generate_path_matcher("std::collections::HashMap");
    let generics = extract_generics(ty, path_matcher);
    match &generics[..] {
        &[first, second] => Some((first, second)),
        _ => None,
    }
}

pub(crate) fn extract_types_from_option(ty: &Type) -> Option<&Type> {
    let path_matcher = generate_path_matcher("std::option::Option");
    let generics = extract_generics(ty, path_matcher);
    match &generics[..] {
        &[first] => Some(first),
        _ => None,
    }
}

pub(crate) fn extract_types_from_vec(ty: &Type) -> Option<&Type> {
    let path_matcher = generate_path_matcher("std::vec::Vec");
    let generics = extract_generics(ty, path_matcher);
    match &generics[..] {
        &[first] => Some(first),
        _ => None,
    }
}

pub(crate) fn is_vec_event(ty: &Type) -> bool {
    let extracted = extract_types_from_vec(ty);
    match extracted {
        Some(t) => is_event(t),
        _ => false,
    }
}

pub(crate) fn is_hashmap_event(ty: &Type) -> bool {
    let extracted = extract_types_from_hashmap(ty);
    let string_matcher = generate_type_matcher("std::string::String");
    match extracted {
        Some((left, right)) => string_matcher(left) && is_event(right),
        _ => false,
    }
}

pub(crate) fn is_option_event(ty: &Type) -> bool {
    let extracted = extract_types_from_option(ty);
    match extracted {
        Some(t) => is_event(t),
        _ => false,
    }
}

pub(crate) fn is_event(ty: &Type) -> bool {
    generate_type_matcher("cloudevent::Event")(ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_matcher() -> Result<(), syn::Error> {
        let path_matcher = generate_path_matcher("std::collections::HashMap");

        assert_eq!(path_matcher(&syn::parse_str("std::collections::HashMap")?), true);
        assert_eq!(path_matcher(&syn::parse_str("collections::HashMap")?), true);
        assert_eq!(path_matcher(&syn::parse_str("HashMap")?), true);

        assert_eq!(path_matcher(&syn::parse_str("std::collections")?), false);
        assert_eq!(path_matcher(&syn::parse_str("std")?), false);
        assert_eq!(path_matcher(&syn::parse_str("Vec")?), false);
        assert_eq!(path_matcher(&syn::parse_str("std::collections::SomethingElse")?), false);
        Ok(())
    }

    #[test]
    fn extract_result_ok() -> Result<(), syn::Error> {
        let parsed_result = &syn::parse_str("Result<A, B>")?;
        let (left,right) = extract_types_from_result(parsed_result).unwrap();
        assert_eq!(
            left,
            &syn::parse_str::<syn::Type>("A")?
        );
        assert_eq!(
            right,
            &syn::parse_str::<syn::Type>("B")?
        );
        Ok(())
    }

    #[test]
    fn extract_result_fail() -> Result<(), syn::Error> {
        assert_eq!(extract_types_from_result(&syn::parse_str("NoResult<A, B>")?), None);
        assert_eq!(extract_types_from_result(&syn::parse_str("Result<A>")?), None);
        Ok(())
    }

    #[test]
    fn extract_hashmap_ok() -> Result<(), syn::Error> {
        let parsed_result = &syn::parse_str("HashMap<A, B>")?;
        let (left,right) = extract_types_from_hashmap(parsed_result).unwrap();
        assert_eq!(
            left,
            &syn::parse_str::<syn::Type>("A")?
        );
        assert_eq!(
            right,
            &syn::parse_str::<syn::Type>("B")?
        );
        Ok(())
    }

    #[test]
    fn extract_hashmap_long_ok() -> Result<(), syn::Error> {
        let parsed_result = &syn::parse_str("std::collections::HashMap<A, B>")?;
        let (left,right) = extract_types_from_hashmap(parsed_result).unwrap();
        assert_eq!(
            left,
            &syn::parse_str::<syn::Type>("A")?
        );
        assert_eq!(
            right,
            &syn::parse_str::<syn::Type>("B")?
        );
        Ok(())
    }

    #[test]
    fn extract_hashmap_fail() -> Result<(), syn::Error> {
        assert_eq!(extract_types_from_hashmap(&syn::parse_str("NoHashMap<A, B>")?), None);
        assert_eq!(extract_types_from_hashmap(&syn::parse_str("HashMap<A>")?), None);
        Ok(())
    }

    #[test]
    fn extract_option_ok() -> Result<(), syn::Error> {
        let parsed_result = &syn::parse_str("Option<A>")?;
        let sub = extract_types_from_option(parsed_result).unwrap();
        assert_eq!(
            sub,
            &syn::parse_str::<syn::Type>("A")?
        );
        Ok(())
    }

    #[test]
    fn extract_option_fail() -> Result<(), syn::Error> {
        assert_eq!(extract_types_from_option(&syn::parse_str("NoOption<A>")?), None);
        assert_eq!(extract_types_from_option(&syn::parse_str("Option<A, B>")?), None);
        Ok(())
    }

    #[test]
    fn extract_vec_ok() -> Result<(), syn::Error> {
        let parsed_result = &syn::parse_str("Vec<A>")?;
        let sub = extract_types_from_vec(parsed_result).unwrap();
        assert_eq!(
            sub,
            &syn::parse_str::<syn::Type>("A")?
        );
        Ok(())
    }

    #[test]
    fn extract_vec_fail() -> Result<(), syn::Error> {
        assert_eq!(extract_types_from_vec(&syn::parse_str("NoVec<A>")?), None);
        assert_eq!(extract_types_from_vec(&syn::parse_str("Vec<A, B>")?), None);
        Ok(())
    }
}
