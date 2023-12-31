use std::path::Path;

use anyhow::Result;
use proc_macro2::Literal;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote, Data, DeriveInput, Fields, GenericParam, Generics};

// Add a bound `T: FromProperty` to every type parameter T.
fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(rma_lib::FromProperty));
        }
    }
    generics
}

#[proc_macro_derive(FromProperty)]
pub fn derive_from_property(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        impl<C: Seek + Read> #impl_generics rma_lib::FromProperty<C> for #name #ty_generics #where_clause {
            fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
                match property {
                    Property::StructProperty(property) => {
                        ::rma_lib::checked_read(asset, &property.value)
                    },
                    _ => ::anyhow::bail!("{property:#?}"),
                }
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

#[proc_macro_derive(FromExport)]
pub fn derive_from_export(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let mut generics = input.generics;
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(rma_lib::FromProperty));
        }
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        impl<C: Seek + Read> #impl_generics rma_lib::FromExport<C> for #name #ty_generics #where_clause {
            fn from_export(asset: &Asset<C>, package_index: PackageIndex) -> Result<Self> {
                let export = ::rma_lib::resolve_package_index(asset, package_index)?;
                let normal_export = export.get_normal_export().expect("export is a NormalExport");
                let properties = &normal_export.properties;

                ::rma_lib::checked_read(asset, properties)
            }
        }
        impl<C: Seek + Read> #impl_generics rma_lib::FromProperty<C> for #name #ty_generics #where_clause {
            fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
                rma_lib::from_object_property(asset, property)
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

#[proc_macro_derive(FromProperties)]
pub fn derive_from_properties(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let mut generics = input.generics;
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(rma_lib::FromProperty));
        }
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let members = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                use heck::ToPascalCase;

                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let name_str = name.as_ref().expect("identifier has a name").to_string();
                    let literal = Literal::string(&name_str.to_pascal_case());

                    if name_str == "base" {
                        quote_spanned! {f.span()=>
                            #name: ::rma_lib::FromProperties::from_properties(asset, properties, expected_properties)?,
                        }
                    } else {
                        quote_spanned! {f.span()=>
                            #name: ::rma_lib::property_or_default_notify(asset, properties, #literal, expected_properties)?,
                        }
                    }
                });
                quote! {
                    #(#recurse)*
                }
            }
            Fields::Unnamed(ref _fields) => {
                unimplemented!();
            }
            Fields::Unit => {
                unimplemented!();
            }
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    };

    let expanded = quote! {
        impl<C: Seek + Read> #impl_generics rma_lib::FromProperties<C> for #name #ty_generics #where_clause {
            fn from_properties(asset: &::unreal_asset::Asset<C>, properties: &[::unreal_asset::properties::Property], expected_properties: &mut ::std::collections::HashSet<&str>) -> Result<Self> {
                Ok(Self {
                    #members
                })
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

fn read_dir_recursive<P: AsRef<Path>>(root: &str, path: P, paths: &mut Vec<String>) -> Result<()> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = std::fs::metadata(path.clone())?;
        if metadata.is_file() {
            let rel = path.strip_prefix(root)?.to_str().unwrap();
            paths.push(rel.replace('\\', "/"));
        } else if metadata.is_dir() {
            read_dir_recursive(root, &path, paths)?;
        } else {
            panic!("{:?} is not a file or directory", entry);
        }
    }
    Ok(())
}

#[proc_macro]
pub fn list_dir(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let literal = input.to_string();
    if !(literal.starts_with('"') && literal.ends_with('"') && literal.len() >= 2) {
        panic!("expected string literal");
    }

    let path = &literal[1..literal.len() - 1];

    let mut paths = vec![];
    read_dir_recursive(path, path, &mut paths).unwrap();
    paths.sort();
    let paths = paths.iter().map(|p| Literal::string(p));

    let expanded = quote! {
        [#(#paths,)*]
    };

    proc_macro::TokenStream::from(expanded)
}
