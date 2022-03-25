use ibig::ubig;
use num_traits::{One, Pow, Zero};

use crate::RBig;

impl Zero for RBig {
	fn zero() -> Self {
		Self::from(ubig!(0))
	}

	fn set_zero(&mut self) {
		self.numer.set_zero()
	}

	fn is_zero(&self) -> bool {
		self.numer.is_zero()
	}
}

impl One for RBig {
	fn one() -> Self {
		Self::from(ubig!(1))
	}

	fn is_one(&self) -> bool
	where
		Self: PartialEq,
	{
		self.sign.is_positive() && &self.numer == self.denom.as_ref()
	}
}

impl Pow<usize> for RBig {
	type Output = Self;

	fn pow(self, rhs: usize) -> Self::Output {
		self.unsigned_pow(rhs)
	}
}

// todo: impl Checked*, {To,From}Primitive, Inv