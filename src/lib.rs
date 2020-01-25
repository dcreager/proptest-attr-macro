// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2019, Douglas Creager.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
// in compliance with the License.  You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the
// License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either
// express or implied.  See the License for the specific language governing permissions and
// limitations under the License.
// ------------------------------------------------------------------------------------------------

//! This crate provides a procedural attribute macro version of [proptest]'s `proptest!` macro.
//!
//! So instead of having to write:
//!
//! ```
//! use proptest::proptest;
//!
//! proptest! {
//!     fn test_excluded_middle(x: u32, y: u32) {
//!         assert!(x == y || x != y);
//!     }
//! }
//! ```
//!
//! you can write:
//!
//! ```
//! use proptest_attr_macro::proptest;
//!
//! #[proptest]
//! fn test_excluded_middle(x: u32, y: u32) {
//!     assert!(x == y || x != y);
//! }
//! ```
//! [proptest]: https://docs.rs/proptest/*/
//!
//! ## Limitations
//!
//! Procedural attribute macros can only be used with valid Rust syntax, which means that you can't
//! use proptest's `in` operator (which allows you to draw values from a specific strategy
//! function):
//!
//! ``` compile_fail
//! // This won't compile!
//! #[proptest]
//! fn test_even_numbers(x in even(any::<u32>())) {
//!     assert!((x % 2) == 0);
//! }
//! ```
//!
//! Instead you must provide an actual parameter list, just like you would with a real Rust
//! function definition.  That, in turn, means that your function parameters can only draw values
//! using the `any` strategy for their types.  If you want to use a custom strategy, you must
//! create a separately named type, and have the new type's `Arbitrary` impl use that strategy:
//!
//! ```
//! # #[derive(Clone, Debug)]
//! struct Even { value: i32 }
//!
//! # use proptest::arbitrary::Arbitrary;
//! # use proptest::strategy::BoxedStrategy;
//! # use proptest::strategy::Strategy;
//! impl Arbitrary for Even {
//!     type Parameters = ();
//!     type Strategy = BoxedStrategy<Even>;
//!
//!     fn arbitrary_with(_args: ()) -> Self::Strategy {
//!         (0..100).prop_map(|x| Even { value: x * 2 }).boxed()
//!     }
//! }
//!
//! # use proptest_attr_macro::proptest;
//! #[proptest]
//! fn test_even_numbers(even: Even) {
//!     assert!((even.value % 2) == 0);
//! }
//! ```
//!
//! ## Benefits
//!
//! The main one is purely aesthetic: since you're applying the `proptest` attribute macro to valid
//! Rust functions, `rustfmt` works on them!

extern crate proc_macro;

use proc_macro::Delimiter::Parenthesis;
use proc_macro::*;
use quote::quote;
use syn::parse_macro_input;
use syn::Item;

/// An attribute macro that marks a function as a test case, and uses proptest's [`any`][] strategy
/// to produce random values for each of the function's parameters.
///
/// [`any`]: https://docs.rs/proptest/*/proptest/prelude/fn.any.html
///
/// ```
/// # extern crate proptest_attr_macro;
/// # use proptest_attr_macro::proptest;
/// #[proptest]
/// fn test_excluded_middle(x: u32, y: u32) {
///     assert!(x == y || x != y);
/// }
/// ```
#[proc_macro_attribute]
pub fn proptest(args: TokenStream, input: TokenStream) -> TokenStream {
    let strategies = split_by_comma(args);

    let input_for_assertion = input.clone();
    let item = parse_macro_input!(input_for_assertion as Item);

    let proptest_input = proc_macro2::TokenStream::from(add_strategies(input, strategies));

    match item {
        Item::Fn(_) => {
            let output = quote! {
                ::proptest::prelude::proptest! {
                    #[test]
                    #proptest_input
                }
            };
            output.into()
        }
        _ => {
            let msg = "#[proptest] is only supported on functions";
            syn::parse::Error::new_spanned(item, msg)
                .to_compile_error()
                .into()
        }
    }
}

fn add_strategies(input: TokenStream, strategies: Vec<Vec<TokenTree>>) -> TokenStream {
    if strategies.is_empty() {
        return input;
    }

    let in_ident: TokenTree = TokenTree::Ident(Ident::new("in", Span::call_site()));

    let mut new_stream = TokenStream::new();

    for tree in input.into_iter() {
        match tree {
            TokenTree::Group(group) => {
                let group = if group.delimiter() == Parenthesis {
                    let arguments = split_by_comma(group.stream());

                    let var_names: Vec<TokenTree> = arguments
                        .into_iter()
                        .map(|args| match &args[..] {
                            [TokenTree::Ident(var), TokenTree::Punct(_), TokenTree::Ident(_ty)] => {
                                TokenTree::Ident(var.clone())
                            }
                            _ => panic!("Unexpected signature {:?}", args),
                        })
                        .collect();

                    let args = var_names
                        .into_iter()
                        .zip(strategies.clone())
                        .map(|(var, strategy)| {
                            let mut v = vec![var, in_ident.clone()];
                            v.extend(strategy);
                            v
                        })
                        .collect::<Vec<_>>();
                    let sl = &args[..];

                    let args = sl.join(&TokenTree::Punct(Punct::new(',', Spacing::Alone)));

                    let mut stream = TokenStream::new();
                    stream.extend(args);
                    Group::new(Parenthesis, stream)
                } else {
                    group
                };
                new_stream.extend(vec![TokenTree::Group(group)]);
            }
            t => new_stream.extend(TokenStream::from(t)),
        }
    }

    new_stream
}

fn split_by_comma(args: TokenStream) -> Vec<Vec<TokenTree>> {
    let args = args.into_iter().collect::<Vec<_>>();

    let strategies: Vec<Vec<TokenTree>> = args
        .split(|t: &TokenTree| match t {
            TokenTree::Punct(p) => &p.to_string() == ",",
            _ => false,
        })
        .map(|v| Vec::from(v))
        .collect::<Vec<_>>();

    strategies
}
