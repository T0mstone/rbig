//! # RBig
//!
//! An arbitrary-size rational number type built on [`ibig`](https://docs.rs/ibig/0.3.4/ibig).

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::mem::swap;
use std::ops::{Mul, Neg};

use ibig::ops::UnsignedAbs;
use ibig::{ibig, ubig, IBig, UBig};
use num_traits::Zero;

use crate::nonzero_ubig::NonZeroUBig;
use crate::util::*;

pub mod reexport {
	pub use ibig;
}
pub mod nonzero_ubig;
#[cfg(feature = "num-traits-impls")]
mod num_traits_impls;
mod util;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Sign {
	Negative,
	Positive,
}

impl Default for Sign {
	#[inline]
	fn default() -> Self {
		Self::Positive
	}
}

impl Sign {
	#[inline]
	pub fn positive_if(b: bool) -> Self {
		if b {
			Self::Positive
		} else {
			Self::Negative
		}
	}

	#[inline]
	pub fn is_positive(self) -> bool {
		matches!(self, Self::Positive)
	}

	pub fn pow(self, exp: usize) -> Self {
		match self {
			Sign::Positive => Sign::Positive,
			Sign::Negative => Self::positive_if(exp % 2 == 0),
		}
	}
}

impl Neg for Sign {
	type Output = Self;

	#[inline]
	fn neg(self) -> Self::Output {
		match self {
			Self::Positive => Self::Negative,
			Self::Negative => Self::Positive,
		}
	}
}

impl Mul for Sign {
	type Output = Self;

	#[inline]
	fn mul(self, rhs: Self) -> Self::Output {
		Self::positive_if(self == rhs)
	}
}

impl Mul<IBig> for Sign {
	type Output = IBig;

	#[inline]
	fn mul(self, rhs: IBig) -> Self::Output {
		match self {
			Sign::Positive => rhs,
			Sign::Negative => -rhs,
		}
	}
}

/// An arbitrary-size rational number
///
/// Zero has a lot of representations, those being `+0/x` and `-0/x` for any unsigned integer `x`.
/// Notice that the sign is allowed to be `-`, but it still represents the same value.
#[derive(Debug, Clone)]
pub struct RBig {
	pub sign: Sign,
	pub numer: UBig,
	pub denom: NonZeroUBig,
}

impl RBig {
	/// Constructs an `RBig` from a sign and the absolute values of the numerator and denominator
	#[inline]
	pub fn new(sign: Sign, numer: UBig, denom: NonZeroUBig) -> Self {
		Self { sign, numer, denom }
	}

	/// Constructs an `RBig` from a signed numerator and denominator
	///
	/// returns `None` if the denominator is zero
	pub fn from_numer_denom(numer: IBig, denom: IBig) -> Option<Self> {
		let is_positive = (numer >= ibig!(0)) == (denom >= ibig!(0));
		Some(Self::new(
			Sign::positive_if(is_positive),
			numer.unsigned_abs(),
			NonZeroUBig::new(denom.unsigned_abs())?,
		))
	}

	/// Constructs an `RBig` from a signed numerator and an unsigned denominator
	pub fn from_numer_unsigned_denom(numer: IBig, denom: NonZeroUBig) -> Self {
		Self::new(
			Sign::positive_if(numer >= ibig!(0)),
			numer.unsigned_abs(),
			denom,
		)
	}
}

impl From<UBig> for RBig {
	#[inline]
	fn from(numer: UBig) -> Self {
		Self::new(Sign::Positive, numer, NonZeroUBig::one())
	}
}

impl From<IBig> for RBig {
	#[inline]
	fn from(numer: IBig) -> Self {
		Self::new(
			Sign::positive_if(numer >= ibig!(0)),
			numer.unsigned_abs(),
			NonZeroUBig::one(),
		)
	}
}

/// helper functions
impl RBig {
	fn cross_mul_abs(self, rhs: RBig) -> Pair<UBig> {
		Pair(self.numer * rhs.denom.get(), rhs.numer * self.denom.get())
	}

	fn cross_mul_signed(self, rhs: RBig) -> Pair<IBig> {
		Pair(
			self.sign * IBig::from(self.numer * rhs.denom.get()),
			rhs.sign * IBig::from(rhs.numer * self.denom.get()),
		)
	}

	fn logical_signum(&self) -> LogicalSignum {
		match self.sign {
			_ if self.numer == ubig!(0) => LogicalSignum::Zero,
			Sign::Positive => LogicalSignum::Pos,
			Sign::Negative => LogicalSignum::Neg,
		}
	}
}

impl PartialEq for RBig {
	fn eq(&self, other: &Self) -> bool {
		if self.numer == ubig!(0) && other.numer == ubig!(0) {
			return true;
		}
		self.sign == other.sign
			&& self
				.clone()
				.cross_mul_abs(other.clone())
				.as_ref()
				.fold(PartialEq::eq)
	}
}

impl Eq for RBig {}

impl PartialOrd for RBig {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for RBig {
	fn cmp(&self, other: &Self) -> Ordering {
		self.logical_signum()
			.cmp(&other.logical_signum())
			.then_with(|| {
				self.clone()
					.cross_mul_abs(other.clone())
					.as_ref()
					.fold(Ord::cmp)
			})
	}
}

impl Hash for RBig {
	fn hash<H: Hasher>(&self, state: &mut H) {
		let reduced = self.clone().reduced();
		let sign = if reduced.numer == ubig!(0) {
			Sign::Positive
		} else {
			reduced.sign
		};
		(sign, reduced.numer, reduced.denom).hash(state)
	}
}

//       /------\
//      /        \
//      | /----\ |-\
//      | \----/ | |
//      |        |-/
//      |  /--\  |
//      \_/    \_/
//
// red is sus
#[allow(clippy::suspicious_arithmetic_impl, clippy::suspicious_op_assign_impl)]
mod arith_impls {
	use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

	use crate::RBig;

	impl Mul for RBig {
		type Output = Self;

		fn mul(self, rhs: Self) -> Self::Output {
			Self {
				sign: self.sign * rhs.sign,
				numer: self.numer * rhs.numer,
				denom: self.denom * rhs.denom,
			}
		}
	}

	impl Div for RBig {
		type Output = Self;

		fn div(self, rhs: Self) -> Self::Output {
			self * rhs.recip()
		}
	}

	impl MulAssign for RBig {
		fn mul_assign(&mut self, rhs: Self) {
			self.sign = self.sign * rhs.sign;
			self.numer *= rhs.numer;
			self.denom *= rhs.denom;
		}
	}

	impl DivAssign for RBig {
		fn div_assign(&mut self, rhs: Self) {
			*self *= rhs.recip();
		}
	}

	impl Add for RBig {
		type Output = Self;

		fn add(self, rhs: Self) -> Self::Output {
			let denom = self.denom.clone() * rhs.denom.clone();
			let numer = self.cross_mul_signed(rhs).fold(std::ops::Add::add);

			Self::from_numer_unsigned_denom(numer, denom)
		}
	}

	impl Sub for RBig {
		type Output = Self;

		fn sub(self, rhs: Self) -> Self::Output {
			let denom = self.denom.clone() * rhs.denom.clone();
			let numer = self.cross_mul_signed(rhs).fold(std::ops::Sub::sub);

			Self::from_numer_unsigned_denom(numer, denom)
		}
	}

	impl AddAssign for RBig {
		fn add_assign(&mut self, rhs: Self) {
			let delta = rhs.numer * self.denom.clone().get();
			self.numer *= rhs.denom.clone().get();
			self.denom *= rhs.denom;
			let rel_sign = rhs.sign * self.sign;
			if rel_sign.is_positive() {
				self.numer += delta;
			} else if self.numer >= delta {
				self.numer -= delta;
			} else {
				self.sign = -self.sign;
				self.numer = delta - std::mem::take(&mut self.numer);
			}
		}
	}

	impl SubAssign for RBig {
		fn sub_assign(&mut self, rhs: Self) {
			*self += -rhs;
		}
	}

	impl Neg for RBig {
		type Output = Self;

		fn neg(mut self) -> Self::Output {
			self.sign = -self.sign;
			self
		}
	}

	// todo: arithmetic with IBig, UBig and with machine integer types
	// todo: `Product`, `Sum` impls
}

impl RBig {
	/// Returns the reciprocal without checking the denominator
	///
	/// # Safety
	/// This is safe if and only if `self.numer != 0`.
	pub unsafe fn unchecked_recip(mut self) -> Self {
		swap(&mut self.numer, self.denom.get_mut());
		self
	}

	/// Returns the reciprocal, or `None` if the numerator is zero
	pub fn checked_recip(self) -> Option<Self> {
		if self.numer == ubig!(0) {
			return None;
		}
		// SAFETY: The early return ensures that `self.numer != 0` holds here.
		Some(unsafe { self.unchecked_recip() })
	}

	/// Returns the reciprocal
	///
	/// # Panics
	/// Panics if `self.numer` is zero
	pub fn recip(self) -> Self {
		assert_ne!(self.numer, ubig!(0), "tried to call `recip` on zero");
		// SAFETY: The assertion ensures that `self.numer != 0` holds here.
		unsafe { self.unchecked_recip() }
	}

	/// Reduces the fraction
	///
	/// Divides both numerator and denominator by their GCD (greatest common divisor)
	pub fn reduce(&mut self) {
		let gcd = gcd(self.numer.clone(), self.denom.clone().get());
		self.numer /= gcd.clone();
		self.denom /= gcd;
	}

	/// Returns the reduced fraction of `self`
	///
	/// Like [`reduce`](Self::reduce), but returns the result instead of modifying `self`
	pub fn reduced(mut self) -> Self {
		self.reduce();
		self
	}

	pub fn is_int(&self) -> bool {
		self.numer.clone() % self.denom.clone().get() == ubig!(0)
	}

	pub fn try_into_int(self) -> Result<IBig, Self> {
		if self.is_int() {
			Ok(self.sign * IBig::from(self.reduced().numer))
		} else {
			Err(self)
		}
	}

	pub fn try_to_int(&self) -> Option<IBig> {
		self.is_int()
			.then(|| self.sign * IBig::from(self.clone().reduced().numer))
	}

	pub fn is_uint(&self) -> bool {
		self.is_int() && !self.is_negative()
	}

	pub fn try_into_uint(self) -> Result<UBig, Self> {
		if self.is_uint() {
			Ok(self.reduced().numer)
		} else {
			Err(self)
		}
	}

	pub fn try_to_uint(&self) -> Option<UBig> {
		self.is_uint().then(|| self.clone().reduced().numer)
	}

	pub fn is_positive(&self) -> bool {
		self.numer != ubig!(0) && self.sign.is_positive()
	}

	pub fn is_negative(&self) -> bool {
		self.numer != ubig!(0) && !self.sign.is_positive()
	}

	pub fn is_zero(&self) -> bool {
		self.numer == ubig!(0)
	}

	/// Returns `0` if `self == 0`, otherwise `+1` or `-1`,
	/// such that the returned value's sign is the same as `self.sign`
	pub fn signum(&self) -> Self {
		if self.is_zero() {
			Self::from(ubig!(0))
		} else {
			Self::from(self.sign * ibig!(1))
		}
	}

	pub fn abs(mut self) -> Self {
		self.sign = Sign::Positive;
		self
	}

	pub fn unsigned_pow(self, exp: usize) -> Self {
		Self {
			sign: self.sign.pow(exp),
			numer: self.numer.pow(exp),
			denom: self.denom.pow(exp),
		}
	}

	pub fn signed_pow(self, exp: isize) -> Self {
		let res = self.unsigned_pow(exp.unsigned_abs());
		if exp.is_positive() {
			res
		} else {
			res.recip()
		}
	}
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RoundingDirection {
	TowardsZero,
	AwayFromZero,
}

pub trait RoundingDirectionDecider {
	fn decide(self, to_round: &RBig) -> RoundingDirection;
}

pub mod rounding {
	use std::cmp::Ordering;

	use super::{RoundingDirection, RoundingDirectionDecider};
	use crate::RBig;

	impl RoundingDirectionDecider for RoundingDirection {
		fn decide(self, _: &RBig) -> RoundingDirection {
			self
		}
	}

	/// Round towards negative infinity
	pub struct Floor;

	pub type TowardsNegativeInfinity = Floor;

	impl RoundingDirectionDecider for Floor {
		fn decide(self, to_round: &RBig) -> RoundingDirection {
			if to_round.is_negative() {
				RoundingDirection::AwayFromZero
			} else {
				RoundingDirection::TowardsZero
			}
		}
	}

	/// Round towards positive infinity
	pub struct Ceil;

	pub type TowardsPositiveInfinity = Ceil;

	impl RoundingDirectionDecider for Ceil {
		fn decide(self, to_round: &RBig) -> RoundingDirection {
			if to_round.is_positive() {
				RoundingDirection::AwayFromZero
			} else {
				RoundingDirection::TowardsZero
			}
		}
	}

	/// Round towards the nearest integer, using `self.tie_breaker` for half-way cases
	pub struct TowardNearest<T: RoundingDirectionDecider> {
		pub tie_breaker: T,
	}

	impl<T: RoundingDirectionDecider> RoundingDirectionDecider for TowardNearest<T> {
		fn decide(self, to_round: &RBig) -> RoundingDirection {
			let fract = to_round.clone()
				- RBig::from(to_round.clone().round(RoundingDirection::TowardsZero));
			let abs_fract = fract.abs();

			// `2a [?] b` is equiv. to `a/b [?] 1/2`
			match (abs_fract.numer * 2u8).cmp(abs_fract.denom.as_ref()) {
				Ordering::Greater => RoundingDirection::AwayFromZero,
				Ordering::Less => RoundingDirection::TowardsZero,
				Ordering::Equal => self.tie_breaker.decide(to_round),
			}
		}
	}

	/// Round towards the nearest even integer (used in bankers' rounding as the tiebreaker for [`TowardNearest`])
	pub struct TowardNearestEven;

	impl RoundingDirectionDecider for TowardNearestEven {
		fn decide(self, to_round: &RBig) -> RoundingDirection {
			if to_round.clone().round_abs(RoundingDirection::TowardsZero) % 2u8 == 0 {
				RoundingDirection::TowardsZero
			} else {
				RoundingDirection::AwayFromZero
			}
		}
	}

	/// Round towards the nearest odd integer
	pub struct TowardNearestOdd;

	impl RoundingDirectionDecider for TowardNearestOdd {
		fn decide(self, to_round: &RBig) -> RoundingDirection {
			if to_round.clone().round_abs(RoundingDirection::TowardsZero) % 2u8 == 1 {
				RoundingDirection::TowardsZero
			} else {
				RoundingDirection::AwayFromZero
			}
		}
	}
}

impl RBig {
	/// Round the absolute value towards zero (or equivalently: towards negative infinity)
	pub fn abs_floor(self) -> UBig {
		self.numer / self.denom.get()
	}

	// note: make sure to only call this when `self.numer != 0`
	fn abs_ceil_impl(self) -> UBig {
		(self.numer - ubig!(1)) / self.denom.get() + ubig!(1)
	}

	/// Round the absolute value away from zero (or equivalently: towards positive infinity)
	pub fn abs_ceil(self) -> UBig {
		if self.numer.is_zero() {
			return ubig!(0);
		}
		self.abs_ceil_impl()
	}

	pub fn round_abs<D: RoundingDirectionDecider>(mut self, dir: D) -> UBig {
		self = self.abs();
		match dir.decide(&self) {
			RoundingDirection::TowardsZero => self.abs_floor(),
			RoundingDirection::AwayFromZero => self.abs_ceil(),
		}
	}

	pub fn round<D: RoundingDirectionDecider>(self, dir: D) -> IBig {
		let sign = self.sign;
		let abs = match dir.decide(&self) {
			RoundingDirection::TowardsZero => self.abs_floor(),
			RoundingDirection::AwayFromZero => self.abs_ceil(),
		};
		sign * IBig::from(abs)
	}

	/// Returns the integer part, with division rounded towards zero
	///
	/// Short for `Self::from(self.round(TowardZero))`
	pub fn trunc(self) -> Self {
		Self::from(self.round(RoundingDirection::TowardsZero))
	}

	/// Returns the fractional part, with division rounded towards zero
	///
	/// Satisfies `self == self.trunc() + self.fract()` (with appropriate cloning, of course)
	pub fn fract(self) -> Self {
		Self::new(self.sign, self.numer % self.denom.clone().get(), self.denom)
	}
}
