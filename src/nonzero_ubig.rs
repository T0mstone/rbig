use std::ops::{Add, Div, DivAssign, Mul, MulAssign};

use ibig::{ubig, UBig};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NonZeroUBig(UBig);

impl NonZeroUBig {
	#[inline]
	pub fn one() -> Self {
		Self(ubig!(1))
	}

	/// # Safety
	/// Safe exactly if `u != 0`
	pub const unsafe fn new_unchecked(u: UBig) -> Self {
		Self(u)
	}

	pub fn new(u: UBig) -> Option<Self> {
		(u != ubig!(0)).then(|| Self(u))
	}

	pub fn get(self) -> UBig {
		self.0
	}

	/// # Safety
	/// The referenced value must not be set to zero
	pub unsafe fn get_mut(&mut self) -> &mut UBig {
		&mut self.0
	}

	pub fn pow(self, exp: usize) -> NonZeroUBig {
		// raising a nonzero value to some power will never produce zero
		Self(self.0.pow(exp))
	}
}

impl From<NonZeroUBig> for UBig {
	fn from(nz: NonZeroUBig) -> Self {
		nz.0
	}
}

impl AsRef<UBig> for NonZeroUBig {
	fn as_ref(&self) -> &UBig {
		&self.0
	}
}

impl Mul for NonZeroUBig {
	type Output = Self;

	fn mul(self, rhs: Self) -> Self::Output {
		// product of two nonzero values is nonzero
		Self(self.0 * rhs.0)
	}
}

impl MulAssign for NonZeroUBig {
	fn mul_assign(&mut self, rhs: Self) {
		self.0 *= rhs.0;
	}
}

impl Div<UBig> for NonZeroUBig {
	type Output = Self;

	fn div(self, rhs: UBig) -> Self::Output {
		// division of a nonzero value by anything is nonzero
		Self(self.0 / rhs)
	}
}

impl DivAssign<UBig> for NonZeroUBig {
	fn div_assign(&mut self, rhs: UBig) {
		self.0 /= rhs;
	}
}

impl Add for NonZeroUBig {
	type Output = Self;

	fn add(self, rhs: Self) -> Self::Output {
		// sum of two nonzero values is nonzero
		Self(self.0 + rhs.0)
	}
}
