extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::visit_mut::VisitMut;
use syn::{
    parse_macro_input, Attribute, Expr, ExprBreak, ExprConst, ExprMacro, ItemConst, ItemStatic,
    Lit, LitInt, LitStr, Macro, MacroDelimiter, Path, PathSegment, Token, Type,
};

struct LiteralReplacer {
    nth: usize,
    fname: Ident,
    release_tweak: bool,
}

impl LiteralReplacer {
    fn replace(&mut self, i: &mut Expr) {
        let expr = std::mem::replace(
            i,
            Expr::Break(ExprBreak {
                attrs: vec![],
                break_token: Default::default(),
                label: None,
                expr: None,
            }),
        );

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
                    expr,
                    Expr::Lit(syn::ExprLit {
                        attrs: vec![],
                        lit: Lit::Str(LitStr::new(&self.fname.to_string(), Span::call_site())),
                    }),
                    Expr::Lit(syn::ExprLit {
                        attrs: vec![],
                        lit: Lit::Int(LitInt::new(&self.nth.to_string(), Span::call_site())),
                    }),
                ]
                .into_iter()
                .collect::<Punctuated<Expr, Token![,]>>()
                .into_token_stream(),
            },
        });

        self.nth += 1;
    }
}

impl VisitMut for LiteralReplacer {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        match *i {
            Expr::Lit(syn::ExprLit {
                lit: Lit::Char(_) | Lit::Int(_) | Lit::Float(_) | Lit::Bool(_) | Lit::Str(_),
                ..
            }) => {
                self.replace(i);
            }
            Expr::Unary(syn::ExprUnary {
                op: syn::UnOp::Neg(_),
                ref expr,
                ..
            }) if matches!(
                &**expr,
                Expr::Lit(syn::ExprLit {
                    lit: Lit::Int(_) | Lit::Float(_),
                    ..
                })
            ) =>
            {
                self.replace(i);
            }
            _ => syn::visit_mut::visit_expr_mut(self, i),
        }
    }

    fn visit_expr_const_mut(&mut self, _: &mut ExprConst) {}

    fn visit_item_const_mut(&mut self, _: &mut ItemConst) {}

    fn visit_item_static_mut(&mut self, _: &mut ItemStatic) {}

    fn visit_type_mut(&mut self, _: &mut Type) {}

    fn visit_attribute_mut(&mut self, _: &mut Attribute) {}
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

    v.into_token_stream().into()
}
