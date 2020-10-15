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

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use quote::ToTokens;
use syn::parse_macro_input;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::FnArg;
use syn::Item;
use syn::Pat;
use syn::Stmt;
use syn::Token;

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
pub fn proptest(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    match item {
        Item::Fn(mut func) => {
            let func_name = &func.sig.ident;
            let mut func_body = func.block.clone();
            if let Some(stmt) = func_body.stmts.last_mut() {
                if let Stmt::Expr(expr) = stmt {
                    // Function body has a return expression, but that's probably a typo.
                    *stmt = Stmt::Semi(expr.clone(), Token![;](Span::call_site()));
                }
            }
            func_body.stmts.push(parse_quote! { return Ok(()); });

            let mut formal_params = TupleList::new();
            let mut actual_params = Punctuated::<_, Comma>::new();
            let mut names = TupleList::new();
            let mut strategies = TupleList::new();
            for arg in func.sig.inputs.iter() {
                if let FnArg::Typed(typed) = arg {
                    if let Pat::Ident(name) = &*typed.pat {
                        let ty = &typed.ty;
                        formal_params.push(name.ident.clone());
                        actual_params.push(name.ident.clone());
                        names.push(name.ident.to_string());
                        strategies.push(quote! { ::proptest::arbitrary::any::<#ty>() });
                    }
                }
            }

            func.attrs.insert(0, parse_quote! { #[test] });
            func.sig.inputs.clear();
            func.block = parse_quote! {{
                let mut config = ::proptest::test_runner::Config::default();
                config.test_name = Some(concat!(module_path!(), "::", stringify!(#func_name)));
                config.source_file = Some(file!());
                let mut runner = ::proptest::test_runner::TestRunner::new(config);
                let names = #names;
                match runner.run(
                    &::proptest::strategy::Strategy::prop_map(
                        #strategies,
                        |values| ::proptest::sugar::NamedArguments(names, values),
                    ),
                    |::proptest::sugar::NamedArguments(_, #formal_params)| {
                        #func_body
                    }
                ) {
                    Ok(_) => (),
                    Err(e) => panic!("{}\n{}", e, runner),
                }
            }};

            func.into_token_stream().into()
        }
        _ => {
            let msg = "#[proptest] is only supported on functions";
            syn::parse::Error::new_spanned(item, msg)
                .to_compile_error()
                .into()
        }
    }
}

#[derive(Debug)]
struct TupleList<T>(Vec<T>);

impl<T> TupleList<T> {
    fn new() -> TupleList<T> {
        TupleList(Vec::new())
    }

    fn push(&mut self, value: T) {
        self.0.push(value);
    }
}

impl<T> ToTokens for TupleList<T>
where
    T: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let mut result = TokenStream2::new();
        for (idx, value) in self.0.iter().rev().enumerate() {
            if idx == 0 {
                value.to_tokens(&mut result);
            } else {
                result = quote! { (#value, #result) };
            }
        }
        result.to_tokens(tokens);
    }
}
