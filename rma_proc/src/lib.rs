use proc_macro2::{Literal, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, parse_quote, Data, DeriveInput, Fields, GenericParam, Generics, Index,
};

#[proc_macro_derive(FromProperty)]
pub fn derive_heap_size(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let sum = heap_size_sum(&input.data, quote! { &property.value });

    let expanded = quote! {
        impl<C: Seek + Read> #impl_generics rma_lib::FromProperty<C> for #name #ty_generics #where_clause {
            fn from_property(asset: &Asset<C>, property: &Property) -> Result<Self> {
                match property {
                    Property::StructProperty(property) => Ok(Self {
                        #sum
                    }),
                    _ => bail!("{property:#?}"),
                }
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

// Add a bound `T: FromProperty` to every type parameter T.
fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(rma_lib::FromProperty));
        }
    }
    generics
}

// Generate an expression to sum up the heap size of each field.
fn heap_size_sum(data: &Data, properties: TokenStream) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let literal = Literal::string(&format!("{}", name.as_ref().unwrap()));
                    quote_spanned! {f.span()=>
                        #name: property_or_default(asset, #properties, #literal)?,
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
    }
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

    let sum = heap_size_sum(&input.data, quote! { properties });

    let expanded = quote! {
        impl<C: Seek + Read> #impl_generics rma_lib::FromExport<C> for #name #ty_generics #where_clause {
            fn from_export(asset: &Asset<C>, package_index: PackageIndex) -> Result<Self> {
                let export = asset.get_export(package_index).unwrap();
                let normal_export = export.get_normal_export().unwrap();
                let properties = &normal_export.properties;
                Ok(Self {
                    #sum
                })
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
