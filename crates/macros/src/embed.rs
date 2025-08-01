use darling::{ast::NestedMeta, FromMeta};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::ItemStruct;

type SynResult<T> = Result<T, syn::Error>;

#[derive(Debug, FromMeta)]
struct Attributes {
    receiver: syn::Type,
    impl_generics: Option<String>,
    embedded: syn::Type,
}

/// Auxiliary function to enter ?-based error propagation.
pub fn embeddable(attr: TokenStream2, item: TokenStream2) -> SynResult<TokenStream2> {
    let item_struct = syn::parse2::<ItemStruct>(item)?;
    let backed_up_struct = item_struct.clone();

    let Attributes {
        receiver,
        impl_generics,
        embedded,
    } = Attributes::from_list(&NestedMeta::parse_meta_list(attr)?)?;
    let impl_generics = syn::parse_str::<syn::Generics>(&impl_generics.unwrap_or_default())?;

    let struct_name = item_struct.ident;

    let field_embedding = item_struct.fields.iter().map(|field| {
        let field_name = &field
            .ident
            .clone()
            .expect("Only named fields are supported");
        quote! {
            #field_name: self.#field_name.embed(&mut synthesizer, stringify!(#field_name))?
        }
    });

    Ok(quote! {
        #backed_up_struct

        impl #impl_generics Embed for #receiver {
            type Embedded = #embedded;

            fn embed(
                &self,
                synthesizer: &mut impl crate::synthesizer::Synthesizer,
                annotation: impl Into<alloc::string::String>,
            ) -> Result<Self::Embedded, halo2_proofs::plonk::Error> {
                let mut synthesizer = synthesizer.namespaced(annotation);
                Ok(#struct_name {
                    #(#field_embedding),*
                })
            }
        }
    })
}
