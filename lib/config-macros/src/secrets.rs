use proc_macro::TokenStream;
use syn::parse_macro_input;
use quote::quote;
use syn::{DeriveInput, Lit, Meta, NestedMeta};


pub fn secret_doc_macro(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Defaults
    let mut id = String::from("");
    let mut short_desc = String::from("");
    let mut description = String::from("");
    let mut example = String::from("");

    // Struct-level attributes
    for attr in &input.attrs {
        if attr.path.is_ident("secret_doc") {
            if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                for nested in &meta_list.nested {
                    if let NestedMeta::Meta(Meta::NameValue(nv)) = nested {
                        let ident = nv.path.get_ident().unwrap().to_string();
                        if let Lit::Str(lit_str) = &nv.lit {
                            match ident.as_str() {
                                "id" => id = lit_str.value(),
                                "short_desc" => short_desc = lit_str.value(),
                                "description" => description = lit_str.value(),
                                "example" => example = lit_str.value(),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    let id_lit = syn::LitStr::new(&id, proc_macro2::Span::call_site());
    let short_desc_lit = syn::LitStr::new(&short_desc, proc_macro2::Span::call_site());
    let description_lit = syn::LitStr::new(&description, proc_macro2::Span::call_site());
    let example_lit = syn::LitStr::new(&example, proc_macro2::Span::call_site());

    let expanded = quote! {
        impl crate::doc::secret::SecretDoc for #name {
            fn id() -> &'static str { #id_lit }
            fn short_desc() -> &'static str { #short_desc_lit }
            fn description() -> &'static str { #description_lit }
            fn example() -> &'static str { #example_lit }
        }

        inventory::submit! {
            crate::doc::secret::SecretDocEntry {
                id: #id_lit,
                short_desc: #short_desc_lit,
                description: #description_lit,
                example: #example_lit,
            }
        }
    };

    TokenStream::from(expanded)
}