//! proc-macros for the API.

#![allow(clippy::missing_docs_in_private_items)]

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, ItemStruct};

mod error;
mod integration_test;
mod id;

/// Define an integration test.
#[proc_macro_error]
#[proc_macro_attribute]
pub fn integration_test(args: TokenStream, test_func: TokenStream) -> TokenStream {
	let args = parse_macro_input!(args as integration_test::Args);
	let test_func = parse_macro_input!(test_func as integration_test::TestFunction);

	match integration_test::expand(args, test_func) {
		Ok(tokens) => tokens,
		Err(error) => error.into_compile_error().into(),
	}
}

/// Creates an "ID" struct.
#[proc_macro_error]
#[proc_macro_attribute]
pub fn id(_args: TokenStream, item: TokenStream) -> TokenStream {
	let item = parse_macro_input!(item as ItemStruct);

	match id::expand(item) {
		Ok(tokens) => tokens,
		Err(error) => error.into_compile_error().into(),
	}
}
