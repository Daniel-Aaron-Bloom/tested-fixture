use std::mem::replace;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote, Attribute, Ident, ItemFn, Result, ReturnType, Token, Type,
    TypeTuple, Visibility,
};

/// Attribute macro applied to a function to turn it into a unit test which is cached
/// as a fixture
///
/// The syntax supported by this macro is:  `attr* vis? ident (: ty)?`
///
/// All attributes and the visibilty level will be applied to the newly declared
/// static fixture `ident`. The type can either be explicitly specified or will
/// be inferred from the return type of the function being annotated.
#[proc_macro_attribute]
pub fn tested_fixture(attr: TokenStream, item: TokenStream) -> TokenStream {
    tested_fixture_helper(attr, item, false)
}

#[doc(hidden)]
#[proc_macro_attribute]
pub fn tested_fixture_doctest(attr: TokenStream, item: TokenStream) -> TokenStream {
    tested_fixture_helper(attr, item, true)
}

struct Attr {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    #[allow(unused)]
    pub colon: Option<Token![:]>,
    pub ty: Option<Type>,
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        let ident = input.call(Ident::parse_any)?;

        let (colon, ty) = if input.peek(Token![:]) {
            (Some(input.parse()?), Some(input.parse()?))
        } else {
            (None, None)
        };

        Ok(Attr {
            attrs,
            vis,
            ident,
            colon,
            ty,
        })
    }
}

fn tested_fixture_helper(attr: TokenStream, item: TokenStream, doctest: bool) -> TokenStream {
    let found_crate =
        crate_name("tested-fixture").expect("tested-fixture is present in `Cargo.toml`");
    let found_crate = match found_crate {
        FoundCrate::Name(name) => Ident::new(&name, Span::call_site()),
        FoundCrate::Itself if doctest => Ident::new("tested_fixture", Span::call_site()),
        FoundCrate::Itself => <Token![crate]>::default().into(),
    };

    let attr = parse_macro_input!(attr as Attr);
    let mut func = parse_macro_input!(item as ItemFn);

    let func_attrs = &func.attrs;
    let func_vis = &func.vis;
    let func_ident = &func.sig.ident;
    let func_body = &func.block;
    let func_out = match replace(&mut func.sig.output, ReturnType::Default) {
        ReturnType::Default => Type::Tuple(TypeTuple {
            paren_token: Default::default(),
            elems: Default::default(),
        }),
        ReturnType::Type(_, ty) => *ty,
    };

    let fixture_attrs = &attr.attrs;
    let fixture_vis = &attr.vis;
    let fixture_ident = &attr.ident;
    let fixture_ty = attr.ty.as_ref().unwrap_or(&func_out);

    func.sig.output = ReturnType::Type(
        Default::default(),
        Box::new(
            parse_quote!(std::result::Result<impl #found_crate::helpers::Unwrap::<#fixture_ty>, impl std::fmt::Debug>),
        ),
    );
    let func_sig = &func.sig;

    let v = quote!(
        #(#fixture_attrs)*
        #[cfg(test)]
        #fixture_vis static #fixture_ident: #found_crate::helpers::Lazy<&#fixture_ty> =
            #found_crate::helpers::Lazy::new(|| #found_crate::helpers::unwrap(#func_ident));

        #(#func_attrs)*
        #[test]
        #func_vis #func_sig {
            static CELL: #found_crate::helpers::OnceCell<
                std::result::Result<
                    #func_out,
                    &str,
                    // std::sync::Mutex<Box<dyn std::any::Any + Send + 'static>>,
                >
            > = #found_crate::helpers::OnceCell::new();

            let result = CELL.get_or_init(|| {
                std::panic::catch_unwind(|| #func_body).map_err(|_| "panicked")
                // std::panic::catch_unwind(|| #func_body).map_err(std::sync::Mutex::new)
            });

            {
                #[allow(unused_imports)]
                use #found_crate::helpers::{Fixer, Fix};

                result.as_ref().map(|x|
                    Fixer(x).fix().map(|x|
                        Fixer(x).fix().map(|x|
                            Fixer(x).fix().map(|x| Fixer(x).fix())
                        )
                    )
                )
            }
        }

    );

    v.into()
}
