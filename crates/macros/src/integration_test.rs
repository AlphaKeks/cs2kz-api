use std::fs;
use std::path::Path as FsPath;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseBuffer};
use syn::spanned::Spanned;
use syn::token::Async;
use syn::{
	Block, Expr, ExprArray, ExprLit, FnArg, Ident, ItemFn, Lit, MetaNameValue, PatType, ReturnType,
	Signature, Type, TypePath, TypeReference,
};

use crate::error::error;

pub(super) fn expand(
	Args { fixtures }: Args,
	TestFunction {
		asyncness: _,
		ident,
		ctx_arg,
		ret_ty,
		body,
	}: TestFunction,
) -> syn::Result<TokenStream> {
	let fixtures = if fixtures.is_empty() {
		quote!()
	} else {
		quote!(::tokio::try_join!(#( ::sqlx::query!(#fixtures).execute(ctx.database()) ),*)?;)
	};

	Ok(quote! {
		#[tokio::test]
		async fn #ident() -> #ret_ty {
			use crate::testing::Context;
			use ::anyhow::Context as _;

			async fn inner(#ctx_arg) -> #ret_ty {
				#body
			}

			let ctx = Context::new().await.context("initialize testing context")?;

			#fixtures

			if let Err(error) = inner(&ctx).await {
				return Err(error.into());
			}

			let test_id = ctx.id();

			ctx
				.cleanup()
				.await
				.with_context(|| format!("test cleanup `{test_id}`"))?;

			Ok(())
		}
	}
	.into())
}

/// Arguments for the `#[integration_test]` macro.
#[derive(Debug, Default)]
pub(super) struct Args {
	/// SQL fixtures to run in addition to the standard migrations.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// #[integration_test(fixtures = ["insert-some-rows", "drop-some-table"])]
	/// async fn my_test(ctx: &Context) -> TestResult {
	///     // ...
	///
	///     Ok(())
	/// }
	/// ```
	fixtures: Vec<String>,
}

impl Parse for Args {
	fn parse(input: &ParseBuffer<'_>) -> syn::Result<Self> {
		if input.is_empty() {
			return Ok(Self::default());
		}

		let MetaNameValue { path, value, .. } = input.parse()?;

		if path.require_ident()? != "fixtures" {
			error!(path, "unrecognized argument");
		}

		let Expr::Array(ExprArray { elems, .. }) = value else {
			error!(value, "`fixtures` must be an array of string literals");
		};

		let fixtures = elems
			.into_iter()
			.try_fold(Vec::new(), |mut fixtures, expr| {
				let Expr::Lit(ExprLit {
					lit: Lit::Str(value),
					..
				}) = expr
				else {
					error!(expr, "invalid fixture; must be string literal");
				};

				let path = FsPath::new("./database/test-fixtures").join(value.value() + ".sql");
				let queries = fs::read_to_string(&path).map_err(|err| {
					syn::Error::new(value.span(), format!("failed to read {path:?}: {err}"))
				})?;

				let queries = queries
					.split(';')
					.map(|query| query.trim())
					.filter(|query| !query.is_empty())
					.map(|query| query.to_owned());

				fixtures.extend(queries);

				Ok(fixtures)
			})?;

		Ok(Self { fixtures })
	}
}

#[derive(Debug)]
pub(super) struct TestFunction {
	#[allow(dead_code)]
	asyncness: Async,
	ident: Ident,
	ctx_arg: PatType,
	ret_ty: Box<Type>,
	body: Box<Block>,
}

impl Parse for TestFunction {
	fn parse(input: &ParseBuffer<'_>) -> syn::Result<Self> {
		let ItemFn {
			sig:
				Signature {
					constness,
					asyncness,
					unsafety,
					abi,
					fn_token: _,
					ident,
					generics,
					paren_token: _,
					inputs,
					variadic,
					output,
				},
			block: body,
			..
		} = input.parse()?;

		if let Some(constness) = constness {
			error!(constness, "integration tests can't be marked `const`");
		}

		let Some(asyncness) = asyncness else {
			error!(asyncness, "integration tests must be marked `async`");
		};

		if let Some(unsafety) = unsafety {
			error!(unsafety, "integration tests can't be marked `unsafe`");
		}

		if let Some(abi) = abi {
			error!(abi, "integration tests can't have a custom ABI");
		}

		if !generics.params.is_empty() {
			error!(
				generics.params,
				"integration tests can't take generic parameters"
			);
		}

		let ctx_arg = inputs
			.first()
			.ok_or_else(|| {
				syn::Error::new(
					inputs.span(),
					"integration tests must take exactly one argument `ctx: &Context`",
				)
			})
			.and_then(|arg| match arg {
				FnArg::Typed(arg) => Ok(arg.clone()),
				FnArg::Receiver(recv) => {
					error!(recv, "integration tests can't take `self` parameters")
				}
			})?;

		let Type::Reference(TypeReference {
			lifetime: None,
			mutability: None,
			elem: ctx_ty,
			..
		}) = ctx_arg.ty.as_ref()
		else {
			error!(
				ctx_arg.ty,
				"integration tests must take exactly one argument `ctx: &Context`"
			);
		};

		let Type::Path(TypePath { ref path, .. }) = ctx_ty.as_ref() else {
			error!(
				ctx_ty,
				"integration tests must take exactly one argument `ctx: &Context`"
			);
		};

		if path.require_ident()? != "Context" {
			error!(
				path,
				"integration tests must take exactly one argument `ctx: &Context`"
			);
		}

		if let Some(variadic) = variadic {
			error!(variadic, "integration tests can't take variadic arguments");
		}

		let ReturnType::Type(_, ret_ty) = output else {
			error!(output, "integration tests must return a `TestResult`");
		};

		let Type::Path(TypePath { ref path, .. }) = ret_ty.as_ref() else {
			error!(ret_ty, "integration tests must return a `TestResult`");
		};

		if path.require_ident()? != "TestResult" {
			error!(path, "integration tests must return a `TestResult`");
		}

		Ok(Self {
			asyncness,
			ident,
			ctx_arg,
			ret_ty,
			body,
		})
	}
}
