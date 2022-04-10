use failure::{ Fail, Backtrace, Context };
use core::fmt;

/// Custom Error Type for the KV project
#[derive(Debug)]
pub struct KVError {
	inner: Context<KVErrorKind>,
}

/// Custom ErrorKind Type for the KV project
#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum KVErrorKind {
	/// Temporary placeholder for real Error Kinds
	#[fail(display = "Error Message")]
	OneVariant,
}

impl KVError {
	/// get the KVErrorKind of the given KVError
	pub fn kind(&self) -> KVErrorKind {
		*self.inner.get_context()
	}
}

impl Fail for KVError {
	fn cause(&self) -> Option<&dyn Fail> {
		self.inner.cause()
	}

	fn backtrace(&self) -> Option<&Backtrace> {
		self.inner.backtrace()
	}
}

impl fmt::Display for KVError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Display::fmt(&self.inner, f)
	}
}

impl From<KVErrorKind> for KVError {
	fn from(kind: KVErrorKind) -> KVError {
		KVError {
			inner: Context::new(kind),
		}
	}
}

impl From<Context<KVErrorKind>> for KVError {
	fn from(context: Context<KVErrorKind>) -> KVError {
		KVError {
			inner: context,
		}
	}
}