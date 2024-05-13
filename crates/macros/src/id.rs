use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Fields, ItemStruct};

use crate::error::error;

pub(super) fn expand(item: ItemStruct) -> syn::Result<TokenStream> {
	let attrs = &item.attrs;
	let vis = &item.vis;
	let name = &item.ident;

	let Fields::Unnamed(fields) = item.fields else {
		error!(item.fields.span(), "only tuple structs are allowed");
	};

	let Some(id_field) = fields.unnamed.first() else {
		error!(fields.unnamed.span(), "struct must have 1 field");
	};

	if fields.unnamed.len() != 1 {
		error!(fields.unnamed.span(), "struct must have 1 field");
	}

	let id_ty = &id_field.ty;
	let id_vis = &id_field.vis;

	Ok(quote! {
		#(#attrs)*
		#[allow(missing_docs, clippy::missing_docs_in_private_items)]
		#[repr(transparent)]
		#[derive(
			Debug,
			Clone,
			Copy,
			PartialEq,
			Eq,
			PartialOrd,
			Ord,
			Hash,
			::derive_more::Display,
			::derive_more::Into,
			::derive_more::From,
			::derive_more::Deref,
			::serde::Serialize,
			::serde::Deserialize,
			::sqlx::Type,
			::utoipa::ToSchema,
		)]
		#[serde(transparent)]
		#[sqlx(transparent)]
		#[display("{_0}")]
		#vis struct #name(#id_vis #id_ty);
	}
	.into())
}
