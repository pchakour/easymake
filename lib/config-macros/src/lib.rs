use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Lit, Meta, NestedMeta};

use crate::secrets::secret_doc_macro;

mod secrets;


#[proc_macro_derive(SecretDoc, attributes(secret_doc))]
pub fn derive_secret_doc(input: TokenStream) -> TokenStream {
    secret_doc_macro(input)
}


#[proc_macro_derive(ActionDoc, attributes(action_doc, action_prop))]
pub fn derive_action_doc(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Defaults
    let mut id = String::from("");
    let mut short_desc = String::from("");
    let mut description = String::from("");
    let mut example = String::from("");

    // Struct-level attributes
    for attr in &input.attrs {
        if attr.path.is_ident("action_doc") {
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

    // Field-level attributes
    let mut property_tokens = Vec::new();
    if let Data::Struct(data_struct) = &input.data {
        if let Fields::Named(fields_named) = &data_struct.fields {
            for field in &fields_named.named {
                let field_name = field.ident.as_ref().unwrap().to_string();
                let mut field_desc = String::new();
                let mut field_required = false;
                let ty = &field.ty;
                let field_type = quote!(#ty).to_string().replace(" ", "");
                for attr in &field.attrs {
                    if attr.path.is_ident("action_prop") {
                        if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                            for nested in &meta_list.nested {
                                if let NestedMeta::Meta(Meta::NameValue(nv)) = nested {
                                    let field_name = nv.path.get_ident().unwrap();
                                    if field_name == "description" {
                                        if let Lit::Str(lit) = &nv.lit {
                                            field_desc = lit.value();
                                        }
                                    } else if field_name == "required" {
                                        if let Lit::Bool(lit) = &nv.lit {
                                            field_required = lit.value();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let field_name_lit = syn::LitStr::new(&field_name, proc_macro2::Span::call_site());
                let field_desc_lit = syn::LitStr::new(&field_desc, proc_macro2::Span::call_site());
                let field_required_lit =
                    syn::LitBool::new(field_required, proc_macro2::Span::call_site());
                let field_type_lit = syn::LitStr::new(&field_type, proc_macro2::Span::call_site());

                property_tokens.push(quote! {
                    crate::doc::action::PropertyDoc {
                        name: #field_name_lit,
                        description: #field_desc_lit,
                        required: #field_required_lit,
                        ty: #field_type_lit,
                    }
                });
            }
        }
    }

    let id_lit = syn::LitStr::new(&id, proc_macro2::Span::call_site());
    let short_desc_lit = syn::LitStr::new(&short_desc, proc_macro2::Span::call_site());
    let description_lit = syn::LitStr::new(&description, proc_macro2::Span::call_site());
    let example_lit = syn::LitStr::new(&example, proc_macro2::Span::call_site());

    let expanded = quote! {
        impl crate::doc::action::ActionDoc for #name {
            fn id() -> &'static str { #id_lit }
            fn short_desc() -> &'static str { #short_desc_lit }
            fn description() -> &'static str { #description_lit }
            fn example() -> &'static str { #example_lit }
        }

        inventory::submit! {
            crate::doc::action::ActionDocEntry {
                id: #id_lit,
                short_desc: #short_desc_lit,
                description: #description_lit,
                example: #example_lit,
                properties: &[
                    #(#property_tokens),*
                ],
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(DocType, attributes(doc_type))]
pub fn derive_doc_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let mut short_desc = "";
    let mut description = "";

    // Parse #[doc_type(short_desc = "...", description = "...")]
    for attr in &input.attrs {
        if attr.path.is_ident("doc_type") {
            if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                for nested in meta_list.nested {
                    if let NestedMeta::Meta(Meta::NameValue(nv)) = nested {
                        if nv.path.is_ident("short_desc") {
                            if let Lit::Str(lit) = nv.lit {
                                short_desc = Box::leak(lit.value().into_boxed_str());
                            }
                        } else if nv.path.is_ident("description") {
                            if let Lit::Str(lit) = nv.lit {
                                description = Box::leak(lit.value().into_boxed_str());
                            }
                        }
                    }
                }
            }
        }
    }

    let name_lit = name.to_string();

    let expanded = quote! {
        impl crate::doc::action::DocType for #name {
            fn entry() -> crate::doc::action::TypeDocEntry {
                crate::doc::action::TypeDocEntry {
                    name: #name_lit,
                    short_desc: #short_desc,
                    description: #description,
                }
            }
        }

        inventory::submit! {
            crate::doc::action::TypeDocEntry {
                name: #name_lit,
                short_desc: #short_desc,
                description: #description,
            }
        }
    };

    TokenStream::from(expanded)
}