use {
	crate::{
		http::problem_details::{ProblemDetails, ProblemType},
		runtime,
	},
	axum::response::{IntoResponse, Response},
	std::{any::type_name, error::Error, fmt, marker::PhantomData},
};

pub(crate) struct HeaderRejection<T: headers::Header>
{
	kind: HeaderRejectionKind,
	ty: PhantomData<T>,
}

enum HeaderRejectionKind
{
	Missing,
	Decode(headers::Error),
}

impl<T: headers::Header> HeaderRejection<T>
{
	pub(super) fn missing() -> Self
	{
		Self { kind: HeaderRejectionKind::Missing, ty: PhantomData }
	}
}

impl<T: headers::Header> From<headers::Error> for HeaderRejection<T>
{
	fn from(error: headers::Error) -> Self
	{
		Self { kind: HeaderRejectionKind::Decode(error), ty: PhantomData }
	}
}

impl<T: headers::Header> fmt::Debug for HeaderRejection<T>
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt.debug_struct("HeaderRejection")
			.field_with("kind", |fmt| match self.kind {
				HeaderRejectionKind::Missing => fmt.write_str("Missing"),
				HeaderRejectionKind::Decode(ref error) => {
					fmt.debug_tuple("Decode").field(error).finish()
				},
			})
			.field("type", &type_name::<T>())
			.field("header", T::name())
			.finish()
	}
}

impl<T: headers::Header> fmt::Display for HeaderRejection<T>
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		write!(fmt, "failed to extract header")?;

		if !runtime::environment::get().is_production() {
			write!(fmt, " of type `{}` ('{}')", type_name::<T>(), T::name())?;
		}

		fmt.write_str(": ")?;

		match self.kind {
			HeaderRejectionKind::Missing => fmt.write_str("missing"),
			HeaderRejectionKind::Decode(ref error) => fmt::Display::fmt(error, fmt),
		}
	}
}

impl<T: headers::Header> Error for HeaderRejection<T>
{
	fn source(&self) -> Option<&(dyn Error + 'static)>
	{
		match self.kind {
			HeaderRejectionKind::Missing => None,
			HeaderRejectionKind::Decode(ref error) => Some(error),
		}
	}
}

impl<T: headers::Header> IntoResponse for HeaderRejection<T>
{
	fn into_response(self) -> Response
	{
		let mut problem_details = ProblemDetails::new(match self.kind {
			HeaderRejectionKind::Missing => ProblemType::MissingHeader,
			HeaderRejectionKind::Decode(_) => ProblemType::InvalidHeader,
		});

		problem_details.add_extension_member("header", T::name().as_str());

		if let HeaderRejectionKind::Decode(ref error) = self.kind {
			problem_details.set_detail(error.to_string());
		}

		problem_details.into_response()
	}
}
