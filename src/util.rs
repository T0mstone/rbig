use std::cmp::min;
use std::mem::swap;

use ibig::UBig;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Pair<T>(pub T, pub T);

impl<T> Pair<T> {
	pub fn as_ref(&self) -> Pair<&T> {
		Pair(&self.0, &self.1)
	}

	pub fn fold<F: FnOnce(T, T) -> U, U>(self, f: F) -> U {
		f(self.0, self.1)
	}
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum LogicalSignum {
	Neg,
	Zero,
	Pos,
}

/// Binary GCD
///
/// adapted from https://web.archive.org/web/20220212233656/https://en.wikipedia.org/wiki/Binary_GCD_algorithm#Implementation
pub fn gcd(mut n: UBig, mut m: UBig) -> UBig {
	// n = (2^i) u, m = (2^j) v
	// gcd((2^i) u, (2^j) v) = (2^k) gcd(u, v) with u, v odd and k = min(i, j)
	// (2^k) is the greatest power of two that divides both n and m
	// this also uses the zero-check performed by `trailing_zeros` to apply the base case (gcd(n, 0) = gcd(0, n) = n)
	let i = match n.trailing_zeros() {
		Some(x) => x,
		None => return m,
	};
	n >>= i;
	let j = match m.trailing_zeros() {
		Some(x) => x,
		None => return n,
	};
	m >>= j;
	let k = min(i, j);

	loop {
		// loop invariant: n and m are odd
		debug_assert!(n.clone() % 2u8 == 1, "invariant broken: n is even ({n})");
		debug_assert!(m.clone() % 2u8 == 1, "invariant broken: m is even ({m})");

		// swap so that n <= m
		if n > m {
			swap(&mut n, &mut m);
		}

		// `gcd(n, m) = gcd(n, m - n)` if n <= m
		m -= n.clone();

		match m.trailing_zeros() {
			// m = 0
			None => {
				// gcd(n, 0) = n
				// this shift represents the multiplication by (2^k) outside of the gcd, as seen above
				return n << k;
			}
			// m != 0
			Some(j) => {
				// special case of above with i = 0:
				// gcd(n, (2^j) v) = gcd(n, v) with n, v odd
				m >>= j;
			}
		}
	}
}
