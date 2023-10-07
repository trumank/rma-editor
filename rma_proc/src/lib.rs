use proc_macro2::{Literal, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, parse_quote, Data, DeriveInput, Fields, GenericParam, Generics, Index,
};

#[proc_macro_derive(FromProperty)]
pub fn derive_heap_size(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = input.ident;

    // Add a bound `T: HeapSize` to every type parameter T.
    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Generate an expression to sum up the heap size of each field.
    let sum = heap_size_sum(&input.data);

    let expanded = quote! {
        // The generated impl.
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

    // Hand the output tokens back to the compiler.
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
fn heap_size_sum(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    // Expands to an expression like
                    //
                    //     0 + self.x.heap_size() + self.y.heap_size() + self.z.heap_size()
                    //
                    // but using fully qualified function call syntax.
                    //
                    // We take some care to use the span of each `syn::Field` as
                    // the span of the corresponding `heap_size_of_children`
                    // call. This way if one of the field types does not
                    // implement `HeapSize` then the compiler's error message
                    // underlines which field it is. An example is shown in the
                    // readme of the parent directory.
                    let recurse = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let literal = Literal::string(&format!("{}", name.as_ref().unwrap()));
                        quote_spanned! {f.span()=>
                            #name: property_or_default(asset, &property.value, #literal)?,
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
            }
        }
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}
