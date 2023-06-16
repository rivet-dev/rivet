pub mod convert;
pub mod fetch;
mod impls;
pub use impls::*;

pub trait ApiTryFrom<T>: Sized {
	/// The type returned in the event of a conversion error.
	type Error;

	/// Performs the conversion.
	fn try_from(value: T) -> Result<Self, Self::Error>;
}

pub trait ApiTryInto<T>: Sized {
	/// The type returned in the event of a conversion error.
	type Error;

	/// Performs the conversion.
	fn try_into(self) -> Result<T, Self::Error>;
}

impl<T, U> ApiTryInto<U> for T
where
	U: ApiTryFrom<T>,
{
	type Error = U::Error;

	fn try_into(self) -> Result<U, U::Error> {
		U::try_from(self)
	}
}

pub trait ApiFrom<T>: Sized {
	/// Performs the conversion.
	fn api_from(value: T) -> Self;
}

pub trait ApiInto<T>: Sized {
	/// Performs the conversion.
	fn api_into(self) -> T;
}

impl<T, U> ApiInto<U> for T
where
	U: ApiFrom<T>,
{
	fn api_into(self) -> U {
		U::api_from(self)
	}
}
