//! Procedural Macros for the `cs2kz-api` crate.

#[macro_use]
extern crate proc_macro_error as _;

#[macro_use]
extern crate quote as _;

mod as_problem_details;

#[proc_macro_error]
#[proc_macro_derive(AsProblemDetails, attributes(problem, problem_type))]
pub fn as_problem_details(item: proc_macro::TokenStream) -> proc_macro::TokenStream
{
	let item = syn::parse_macro_input!(item as syn::DeriveInput);

	match as_problem_details::expand(item) {
		Ok(tokens) => tokens.into(),
		Err(error) => error.into_compile_error().into(),
	}
}
