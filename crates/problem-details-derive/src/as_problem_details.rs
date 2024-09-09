use darling::{FromDeriveInput, FromVariant};
use syn::{Attribute, DeriveInput, Ident, Meta, MetaList, Path};

pub fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream>
{
	let AsProblemDetails { ident, attrs, data } =
		dbg!(AsProblemDetails::from_derive_input(&input))?;

	let data = data
		.take_enum()
		.ok_or_else(|| syn::Error::new(ident.span(), "must be enum"))?;

	let problem_type = attrs
		.iter()
		.find_map(|attr| match &attr.meta {
			Meta::List(MetaList {
				path: Path { segments, .. },
				tokens,
				..
			}) => segments
				.first()
				.map(|segment| &segment.ident)
				.filter(|ident| *ident == "problem_type")
				.map(|_| tokens),

			_ => None,
		})
		.cloned()
		.ok_or_else(|| syn::Error::new(ident.span(), "missing `problem_type` attribute"))
		.and_then(syn::parse2::<Path>)?;

	let variant_idents = data
		.iter()
		.map(|variant| &variant.ident)
		.collect::<Vec<_>>();

	let tokens = quote! {
		#[automatically_derived]
		impl ::problem_details::AsProblemDetails for #ident {
			type ProblemType = #problem_type;

			fn problem_type(&self) -> Self::ProblemType {
				match *self {
					#(Self::#variant_idents => <Self::ProblemType>::#problem_types,)*
				}
			}

			fn add_extension_members(&self, extension_members: &mut ::problem_details::ExtensionMembers) {
				let _ = extension_members;
			}

			fn detail(&self) -> ::std::borrow::Cow<'static, ::std::primitive::str> {
				match *self {
					#(Self::#variant_idents => #details,)*
				}
			}
		}
	};

	Ok(tokens)
}

#[derive(Debug, FromDeriveInput)]
#[darling(
	forward_attrs(doc, cfg, problem_type),
	supports(struct_newtype, enum_any)
)]
struct AsProblemDetails
{
	ident: Ident,
	attrs: Vec<Attribute>,
	data: darling::ast::Data<Variant, ()>,
}

#[derive(Debug, FromVariant)]
#[darling(attributes(problem), forward_attrs(doc, cfg, error))]
struct Variant
{
	ident: Ident,

	#[darling(default)]
	transparent: bool,
	problem_type: Option<Ident>,

	error: proc_macro2::TokenStream,
}
