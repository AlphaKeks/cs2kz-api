macro_rules! error {
	($token:expr, $($msg:tt)*) => {
		return Err(syn::Error::new($token.span(), format!($($msg)*)))
	};
}

pub(crate) use error;
