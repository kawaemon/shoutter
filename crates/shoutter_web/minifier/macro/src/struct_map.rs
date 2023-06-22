use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Brace;
use syn::{braced, parenthesized, parse_macro_input, Block, Result, Token};

enum Mapping {
    Move { from: Ident, to: Ident },
    Map { to: Ident, b: Block },
}

impl Parse for Mapping {
    fn parse(input: ParseStream) -> Result<Self> {
        let to = input.parse()?;

        if input.peek(Token![:]) {
            let _: Token![:] = input.parse()?;
            if input.peek(Brace) {
                return Ok(Mapping::Map {
                    to,
                    b: input.parse()?,
                });
            }
            return Ok(Mapping::Move {
                from: input.parse()?,
                to,
            });
        }

        Ok(Mapping::Move {
            from: to.clone(),
            to,
        })
    }
}

struct StructMap {
    name: Ident,
    from: Ident,
    from_bind: Ident,
    mappings: Punctuated<Mapping, Token![,]>,
}

impl Parse for StructMap {
    fn parse(input: ParseStream) -> Result<Self> {
        let _: Token![fn] = input.parse()?;
        let name = input.parse()?;

        let from;
        parenthesized!(from in input);

        let from_bind = from.parse()?;
        let _: Token![:] = from.parse()?;
        let from = from.parse()?;

        let mappings;
        braced!(mappings in input);
        let mappings = mappings.parse_terminated(Mapping::parse, Token![,])?;

        Ok(StructMap {
            name,
            from_bind,
            from,
            mappings,
        })
    }
}

struct StructMaps {
    maps: Vec<StructMap>,
}

impl Parse for StructMaps {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut maps = vec![];
        while input.peek(Token![fn]) {
            maps.push(input.parse()?);
        }
        Ok(Self { maps })
    }
}

pub(crate) fn struct_map(input: TokenStream) -> TokenStream {
    let StructMaps { maps } = parse_macro_input!(input as _);
    let mut res = quote!();

    for map in maps {
        let StructMap {
            name,
            from,
            mappings,
            from_bind,
            ..
        } = map;

        let mut fields = quote!();
        for map in mappings {
            match map {
                Mapping::Move { from, to } => {
                    fields = quote! {
                        #fields
                        #to: #from_bind.#from,
                    }
                }
                Mapping::Map { to, b } => {
                    fields = quote! {
                        #fields
                        #to: #b,
                    }
                }
            }
        }

        res = quote! {
            #res
            fn #name(#from_bind: wasmparser::#from) -> wasm_encoder::#from {
                wasm_encoder::#from {
                    #fields
                }
            }
        };
    }

    res.into()
}
