use std::mem;

use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::{self, Punctuated};
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{
	AngleBracketedGenericArguments, AttrStyle, Attribute, Expr, ExprLit, ExprTuple, FnArg,
	GenericArgument, Ident, ItemFn, Lit, LitInt, LitStr, MacroDelimiter, Meta, MetaList,
	MetaNameValue, Path, PathArguments, PathSegment, Type, TypePath,
};

use crate::error;

pub fn expand(
	HandlerArgs { tag, path, responses }: HandlerArgs,
	handler_function: ItemFn,
) -> syn::Result<TokenStream> {
	let method = &handler_function.sig.ident;
	let mut utoipa_args = MetaList {
		path: Path {
			leading_colon: None,
			segments: Punctuated::from_iter([
				PathSegment::from(Ident::new("utoipa", Span::call_site().into())),
				PathSegment::from(Ident::new("path", Span::call_site().into())),
			]),
		},
		delimiter: MacroDelimiter::Paren(Default::default()),
		tokens: quote!(#method, tag = #tag, path = #path),
	};

	let path_param = handler_function
		.sig
		.inputs
		.iter()
		.filter_map(|arg| match arg {
			FnArg::Receiver(_) => None,
			FnArg::Typed(arg) => Some(arg.ty.as_ref()),
		})
		.filter_map(|arg_type| match arg_type {
			Type::Path(TypePath { path, .. }) => Some(path),
			_ => None,
		})
		.filter_map(|path| path.segments.first())
		.filter(|seg| seg.ident == "Path")
		.filter_map(|seg| match seg.arguments {
			PathArguments::AngleBracketed(AngleBracketedGenericArguments { ref args, .. }) => {
				args.first()
			}
			PathArguments::None | PathArguments::Parenthesized(_) => None,
		})
		.filter_map(|path_ty| match path_ty {
			GenericArgument::Type(Type::Path(ty)) => Some(&ty.path.segments),
			_ => None,
		})
		.next();

	let mut responses_meta = MetaList {
		path: Path {
			leading_colon: None,
			segments: Punctuated::from_iter([PathSegment::from(Ident::new(
				"responses",
				Span::call_site().into(),
			))]),
		},
		delimiter: MacroDelimiter::Paren(Default::default()),
		tokens: Default::default(),
	};

	if let Some(param) = path_param {
		let old_tokens = mem::take(&mut utoipa_args.tokens);
		utoipa_args.tokens = quote!(#old_tokens, params(#param));

		responses_meta.tokens = quote!(crate::responses::BadRequest);
	}

	let return_type = match handler_function.sig.output {
		syn::ReturnType::Default => quote!(()),
		syn::ReturnType::Type(_, ref ty) => match ty.as_ref() {
			Type::Path(TypePath { path, .. }) => path
				.segments
				.first()
				.filter(|seg| seg.ident == "Result")
				.map(|seg| &seg.arguments)
				.and_then(|args| match args {
					PathArguments::None | PathArguments::Parenthesized(_) => None,
					PathArguments::AngleBracketed(AngleBracketedGenericArguments {
						args, ..
					}) => args.first().and_then(|arg| match arg {
						GenericArgument::Type(Type::Path(TypePath { path, .. })) => {
							path.segments.first().map(|seg| {
								if seg.ident == "Json" {
									match &seg.arguments {
										args @ PathArguments::None => quote!(#args),
										PathArguments::AngleBracketed(
											AngleBracketedGenericArguments { args, .. },
										) => quote!(#args),
										PathArguments::Parenthesized(args) => quote!(#args),
									}
								} else {
									quote!(#ty)
								}
							})
						}
						GenericArgument::Type(ty) => Some(quote!(#ty)),
						_ => None,
					}),
				})
				.unwrap_or_default(),
			ty => quote!(#ty),
		},
	};

	let mut returns_something = false;
	let mut no_content_set = false;

	for (response, token) in responses {
		let old_tokens = mem::take(&mut responses_meta.tokens);

		if returns_something && !no_content_set {
			responses_meta.tokens = quote!(crate::responses::NoContent, #old_tokens);
			no_content_set = true;
			continue;
		}

		responses_meta.tokens = match response {
			200 => {
				returns_something = true;
				quote!(crate::responses::Ok<#return_type>, #old_tokens)
			}
			201 => {
				returns_something = true;
				quote!(crate::responses::Created<#return_type>, #old_tokens)
			}
			// TODO: remove
			303 => quote!(crate::responses::SeeOther, #old_tokens),
			// TODO: remove
			401 => quote!(#old_tokens, crate::responses::Unauthorized),
			409 => quote!(#old_tokens, crate::responses::Conflict),
			// TODO: remove
			422 => quote!(#old_tokens, crate::responses::UnprocessableEntity),
			502 => quote!(#old_tokens, crate::responses::BadGateway),
			unknown => error!(token, "unknown response code `{unknown}`"),
		};
	}

	{
		let old_tokens = mem::take(&mut utoipa_args.tokens);
		utoipa_args.tokens = quote!(#old_tokens, #responses_meta);
	}

	let utoipa = Attribute {
		pound_token: Default::default(),
		style: AttrStyle::Outer,
		bracket_token: Default::default(),
		meta: Meta::List(utoipa_args),
	};

	let result = quote! {
		#[tracing::instrument(level = "debug")]
		#utoipa
		#handler_function
	};

	println!("{result}");

	Ok(result.into())
}

pub struct HandlerArgs {
	tag: LitStr,
	path: LitStr,
	responses: Vec<(u16, LitInt)>,
}

impl Parse for HandlerArgs {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let metas = Punctuated::<MetaNameValue, Comma>::parse_terminated(input)?;
		let mut metas_iter = metas.iter();

		let tag = parse_handler_arg(&metas, &mut metas_iter, "tag")?;
		let Expr::Lit(ExprLit { lit: Lit::Str(tag), .. }) = tag else {
			error!(tag, "expected string literal");
		};

		let path = parse_handler_arg(&metas, &mut metas_iter, "path")?;
		let Expr::Lit(ExprLit { lit: Lit::Str(path), .. }) = path else {
			error!(path.span(), "expected string literal");
		};

		let responses = parse_handler_arg(&metas, &mut metas_iter, "responses")
			.and_then(|expr| match expr {
				Expr::Tuple(ExprTuple { elems, .. }) => Ok(elems),
				expr @ Expr::Lit(ExprLit { lit: Lit::Int(_), .. }) => {
					Ok(Punctuated::from_iter([expr]))
				}
				other => error!(other, "expected tuple of integers"),
			})?
			.into_iter()
			.map(|value| match value {
				Expr::Lit(ExprLit { lit: Lit::Int(lit), .. }) => Ok(lit),
				other => error!(other, "only integers allowed"),
			})
			.map(|lit| lit.and_then(|lit| lit.base10_parse::<u16>().map(|int| (int, lit))))
			.collect::<syn::Result<_>>()?;

		Ok(Self { tag, path, responses })
	}
}

fn parse_handler_arg(
	metas: &Punctuated<MetaNameValue, Comma>,
	metas_iter: &mut punctuated::Iter<MetaNameValue>,
	name: &str,
) -> syn::Result<Expr> {
	let Some(MetaNameValue { path, value, .. }) = metas_iter.next() else {
		error!(metas, "missing argument");
	};

	if path.require_ident()? != name {
		error!(path, "expected `{name}` param");
	}

	Ok(value.clone())
}
