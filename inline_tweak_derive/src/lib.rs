extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::visit_mut::VisitMut;
use syn::{
    parse_macro_input, Expr, ExprBreak, ExprMacro, Lit, LitInt, LitStr, Macro, MacroDelimiter,
    Path, PathSegment, Token,
};

/*
fn find_and_replace_attributes(
    nth: &mut u32,
    fname: &Literal,
    input: proc_macro::token_stream::IntoIter,
) -> TokenStream {
    TokenStream::from_iter(input.into_iter().map(|inp| match inp {
        TokenTree::Literal(lit) => {
            let lit_v = lit.to_string();

            // Authorize numbers, booleans, and chars
            if !(lit_v.chars().all(|c| match c {
                '0'..='9' | '.' | 'e' | 'E' | '-' | '+' | '_' => true,
                _ => false,
            }) || lit_v == "true"
                || lit_v == "false"
                || lit_v.starts_with('\''))
            {
                return TokenTree::Literal(lit);
            }

            let g = TokenTree::Group(Group::new(
                Delimiter::None,
                TokenStream::from_iter([
                    TokenTree::Ident(Ident::new("inline_tweak", Span::call_site())),
                    TokenTree::Punct(Punct::new(':', proc_macro::Spacing::Joint)),
                    TokenTree::Punct(Punct::new(':', proc_macro::Spacing::Alone)),
                    TokenTree::Ident(Ident::new("derive_tweak", Span::call_site())),
                    TokenTree::Punct(Punct::new('!', proc_macro::Spacing::Alone)),
                    TokenTree::Group(Group::new(
                        Delimiter::Parenthesis,
                        TokenStream::from_iter([
                            TokenTree::Literal(lit),
                            TokenTree::Punct(Punct::new(',', proc_macro::Spacing::Alone)),
                            TokenTree::Literal(Literal::clone(fname)),
                            TokenTree::Punct(Punct::new(',', proc_macro::Spacing::Alone)),
                            TokenTree::Literal(Literal::u32_suffixed(*nth)),
                        ]),
                    )),
                ]),
            ));
            *nth += 1;
            g
        }
        TokenTree::Group(group) => TokenTree::Group(Group::new(
            group.delimiter(),
            find_and_replace_attributes(nth, fname, group.stream().into_iter()),
        )),
        _ => inp,
    }))
}*/

struct LiteralReplacer {
    nth: usize,
    fname: Ident,
    release_tweak: bool,
}

impl VisitMut for LiteralReplacer {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        match *i {
            Expr::Lit(_) => {
                let lit = std::mem::replace(
                    i,
                    Expr::Break(ExprBreak {
                        attrs: vec![],
                        break_token: Default::default(),
                        label: None,
                        expr: None,
                    }),
                );

                let Expr::Lit(lit) = lit else {
                    unreachable!();
                };

                *i = Expr::Macro(ExprMacro {
                    attrs: vec![],
                    mac: Macro {
                        path: Path {
                            segments: [
                                PathSegment::from(Ident::new("inline_tweak", Span::call_site())),
                                PathSegment::from(Ident::new(
                                    if self.release_tweak {
                                        "derive_release_tweak"
                                    } else {
                                        "derive_tweak"
                                    },
                                    Span::call_site(),
                                )),
                            ]
                            .into_iter()
                            .collect(),
                            leading_colon: Some(Default::default()),
                        },
                        bang_token: Default::default(),
                        delimiter: MacroDelimiter::Paren(Default::default()),
                        tokens: [
                            lit.lit,
                            Lit::Str(LitStr::new(&*self.fname.to_string(), Span::call_site())),
                            Lit::Int(LitInt::new(&self.nth.to_string(), Span::call_site())),
                        ]
                        .into_iter()
                        .collect::<Punctuated<Lit, Token![,]>>()
                        .into_token_stream(),
                    },
                });

                self.nth += 1;
            }
            _ => syn::visit_mut::visit_expr_mut(self, i),
        }
    }
}

/// Makes all the number/bool/char literals in a function tweakable.  
/// Doesn't apply to literals in macros as they cannot be replaced by expressions reliably. (e.g in calls to println!)
///
/// # Examples
///
/// ```rust
/// # use std::time::Duration;
/// #[inline_tweak::tweak_fn]
/// fn main() {
///     loop {
///         let v = 1.0; // Try changing this value!
///         println!("{}", v);
///         std::thread::sleep(Duration::from_millis(200)); // or even this value :)
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn tweak_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    do_fn(item, false)
}

/// Makes all the number/bool/char literals in a function tweakable.  
/// Doesn't apply to literals in macros as they cannot be replaced by expressions reliably. (e.g in calls to println!)
///
/// # Examples
///
/// ```rust
/// # use std::time::Duration;
/// #[inline_tweak::tweak_fn]
/// fn main() {
///     loop {
///         let v = 1.0; // Try changing this value!
///         println!("{}", v);
///         std::thread::sleep(Duration::from_millis(200)); // or even this value :)
///     }
/// }
/// ```#[cfg(feature = "release_tweak")]
#[proc_macro_attribute]
pub fn release_tweak_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    do_fn(item, true)
}

fn do_fn(item: TokenStream, release_tweak: bool) -> TokenStream {
    let mut v: syn::ItemFn = parse_macro_input!(item as syn::ItemFn);

    let fname = v.sig.ident.clone();

    LiteralReplacer {
        nth: 0,
        fname,
        release_tweak,
    }
    .visit_item_fn_mut(&mut v);

    let output = quote::quote! {
        #v
    };

    TokenStream::from(output)
}
