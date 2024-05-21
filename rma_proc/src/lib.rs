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

#[proc_macro_derive(ToProperty)]
pub fn derive_to_property(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let literal = Literal::string(&name.to_string()[1..]);

    let expanded = quote! {
        impl<C: Seek + Read> #impl_generics rma_lib::ToProperty<C> for #name #ty_generics #where_clause {
            fn get_type() -> Option<&'static str> {
                Some("StructProperty") // TODO actually?
            }
            fn to_property(&self, ctx: &mut CtxSer<C>, name: ::unreal_asset::types::FName, ancestry: ::unreal_asset::unversioned::Ancestry) -> Result<Option<Property>> {
                let properties = ::rma_lib::ToProperties::to_properties(self, ctx, ancestry.clone())?;
                Ok(Some(::unreal_asset::properties::struct_property::StructProperty {
                    name,
                    ancestry,
                    struct_type: Some(ctx.asset.add_fname(#literal)),
                    struct_guid: Some(Default::default()),
                    property_guid: None,
                    duplication_index: 0,
                    serialize_none: true,
                    value: properties,
                }.into()))
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

#[proc_macro_derive(ToExport)]
pub fn derive_to_export(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
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
        impl<C: Seek + Read> #impl_generics rma_lib::ToExport<C> for #name #ty_generics #where_clause {
            fn to_export(&self, ctx: &mut ::rma_lib::CtxSer<C>) -> Result<PackageIndex> {
                let ancestry = ::unreal_asset::unversioned::Ancestry::new(ctx.asset.add_fname("TODO"));

                let (
                    mut base_export,
                    properties,
                    serialization_before_create_dependencies,
                    new_exports,
                ) = {
                    let mut new_ctx = ::rma_lib::CtxSer::new(ctx.asset, ctx.name_counter);
                    (
                        ::rma_lib::BaseExportGetter::base_export(self, &mut new_ctx)?,
                        ::rma_lib::ToProperties::to_properties(self, &mut new_ctx, ancestry)?,
                        new_ctx.serialization_before_create_dependencies,
                        new_ctx.new_exports,
                    )
                };

                let pi = unreal_asset::types::PackageIndex {
                    index: ctx.asset.asset_data.exports.len() as i32 + 1
                };

                base_export.serialization_before_create_dependencies.extend(serialization_before_create_dependencies);
                base_export.create_before_serialization_dependencies.extend(new_exports);

                ctx.new_exports.push(pi);

                ctx.asset.asset_data.exports.push(::unreal_asset::exports::NormalExport {
                    base_export,
                    extras: vec![0, 0, 0, 0],
                    properties,
                }.into());

                Ok(pi)
            }
        }
        impl<C: Seek + Read> #impl_generics rma_lib::ToProperty<C> for #name #ty_generics #where_clause {
            fn get_type() -> Option<&'static str> {
                todo!("ToExport get_type");
            }
            fn to_property(&self, ctx: &mut ::rma_lib::CtxSer<C>, name: ::unreal_asset::types::FName, ancestry: ::unreal_asset::unversioned::Ancestry) -> Result<Option<Property>> {
                Ok(Some(
                    ::unreal_asset::properties::object_property::ObjectProperty {
                        name,
                        ancestry,
                        property_guid: None,
                        duplication_index: 0,
                        value: self.to_export(ctx)?,
                    }
                    .into(),
                ))
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

#[proc_macro_derive(ToProperties)]
pub fn derive_to_properties(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let mut generics = input.generics;
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(rma_lib::FromProperty));
        }
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut base = None;

    let members = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                use heck::ToPascalCase;

                let recurse = fields.named.iter().filter_map(|f| {
                    let name = &f.ident;
                    let name_str = name.as_ref().expect("identifier has a name").to_string();
                    let literal = Literal::string(&name_str.to_pascal_case());

                    if name_str == "base" {
                        base = Some(quote_spanned! {f.span()=>
                            props.extend(::rma_lib::ToProperties::to_properties(&self.#name, ctx, ancestry.clone())?);
                        });
                        None
                    } else {
                        Some(quote_spanned! {f.span()=>
                            let name = ctx.asset.add_fname(#literal);
                            if let Some(next) = rma_lib::ToProperty::to_property(&self.#name, ctx, name, ancestry.clone())? {
                                props.push(next);
                            }
                        })
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
        impl<C: Seek + Read> #impl_generics ::rma_lib::ToProperties<C> for #name #ty_generics #where_clause {
            fn to_properties(&self, ctx: &mut ::rma_lib::CtxSer<C>, ancestry: ::unreal_asset::unversioned::Ancestry) -> Result<Vec<::unreal_asset::properties::Property>> {
                let mut props: Vec<::unreal_asset::properties::Property> = vec![];
                #members
                #base
                Ok(props)
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
