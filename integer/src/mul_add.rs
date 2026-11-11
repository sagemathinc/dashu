//! Fused multiply-add in place: `x += a * b` and `x -= a * b` without a
//! temporary for the product.
//!
//! When the accumulator already owns a heap buffer (the common case in loops
//! that accumulate many products into the same numbers), the product is added
//! into that buffer directly by the multiplication kernels, which already
//! compute `c += sign * a * b`. Only a carry or a borrow is propagated above
//! the product's length; a borrow out of the top word means the result changed
//! sign, and the buffer is negated in two's complement.

use crate::{
    add,
    arch::word::{SignedWord, Word},
    buffer::Buffer,
    ibig::IBig,
    memory::MemoryAllocation,
    mul,
    primitive::split_dword,
    repr::{
        Repr,
        TypedRepr::{Large, Small},
        TypedReprRef::{self, RefLarge, RefSmall},
    },
    ubig::UBig,
    Sign::{self, *},
};
use core::mem;

impl UBig {
    /// Add the product `a * b` to `self` in place (`self += a * b`).
    ///
    /// The result equals `self + a * b`; when `self` owns a heap buffer, the
    /// product is accumulated into it without allocating a temporary.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// let mut x = UBig::from(10u8);
    /// x.add_mul_assign(&UBig::from(3u8), &UBig::from(4u8));
    /// assert_eq!(x, UBig::from(22u8));
    ///
    /// let big = UBig::ONE << 200;
    /// let mut y = big.clone();
    /// y.add_mul_assign(&big, &UBig::from(5u8));
    /// assert_eq!(y, &big * UBig::from(6u8));
    /// ```
    #[inline]
    pub fn add_mul_assign(&mut self, a: &UBig, b: &UBig) {
        let repr = mem::replace(&mut self.0, Repr::zero());
        self.0 = add_signed_mul_repr(repr, Positive, a.repr(), b.repr());
    }
}

impl IBig {
    /// Add the product `a * b` to `self` in place (`self += a * b`).
    ///
    /// The result equals `self + a * b`; when `self` owns a heap buffer, the
    /// product is accumulated into it without allocating a temporary.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// let mut x = IBig::from(-10);
    /// x.add_mul_assign(&IBig::from(3), &IBig::from(-4));
    /// assert_eq!(x, IBig::from(-22));
    ///
    /// let big = IBig::ONE << 200;
    /// let mut y = big.clone();
    /// y.add_mul_assign(&big, &IBig::from(-3));
    /// assert_eq!(y, &big * IBig::from(-2));
    /// ```
    #[inline]
    pub fn add_mul_assign(&mut self, a: &IBig, b: &IBig) {
        let (sa, ra) = a.as_sign_repr();
        let (sb, rb) = b.as_sign_repr();
        let repr = mem::replace(&mut self.0, Repr::zero());
        self.0 = add_signed_mul_repr(repr, sa * sb, ra, rb);
    }

    /// Subtract the product `a * b` from `self` in place (`self -= a * b`).
    ///
    /// The result equals `self - a * b`; when `self` owns a heap buffer, the
    /// product is subtracted from it without allocating a temporary.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// let mut x = IBig::from(10);
    /// x.sub_mul_assign(&IBig::from(3), &IBig::from(4));
    /// assert_eq!(x, IBig::from(-2));
    ///
    /// let big = IBig::ONE << 200;
    /// let mut y = &big * IBig::from(7);
    /// y.sub_mul_assign(&big, &IBig::from(7));
    /// assert_eq!(y, IBig::ZERO);
    /// ```
    #[inline]
    pub fn sub_mul_assign(&mut self, a: &IBig, b: &IBig) {
        let (sa, ra) = a.as_sign_repr();
        let (sb, rb) = b.as_sign_repr();
        let repr = mem::replace(&mut self.0, Repr::zero());
        self.0 = add_signed_mul_repr(repr, -(sa * sb), ra, rb);
    }
}

/// The words of a nonzero magnitude, without leading zeros; small values
/// are spilled into `spill`.
#[inline]
fn magnitude_words<'a>(r: TypedReprRef<'a>, spill: &'a mut [Word; 2]) -> &'a [Word] {
    match r {
        RefSmall(dw) => {
            let (lo, hi) = split_dword(dw);
            *spill = [lo, hi];
            if hi == 0 {
                &spill[..1]
            } else {
                &spill[..]
            }
        }
        RefLarge(words) => words,
    }
}

/// Two's complement negation of a nonzero word sequence, in place.
fn negate_in_place(words: &mut [Word]) {
    let mut carry = true;
    for w in words.iter_mut() {
        let (v, c) = (!*w).overflowing_add(carry as Word);
        *w = v;
        carry = c;
    }
    debug_assert!(!carry, "the negated value was zero");
}

/// `x + sign * |a| * |b|`, where `x` is a signed representation, reusing
/// `x`'s heap buffer when it has one.
fn add_signed_mul_repr(x: Repr, sign: Sign, a: TypedReprRef, b: TypedReprRef) -> Repr {
    let a_zero = matches!(a, RefSmall(0));
    let b_zero = matches!(b, RefSmall(0));
    if a_zero || b_zero {
        return x;
    }
    let (xsign, x) = x.into_sign_typed();
    let mut buffer = match x {
        // a small accumulator and a small product: the ordinary operators
        // stay on the stack
        Small(dw) if matches!((a, b), (RefSmall(_), RefSmall(_))) => {
            let x = Repr::from_dword(dw).with_sign(xsign);
            let product = a * b;
            let product = product.with_sign(sign);
            return add_signed_repr(x, product);
        }
        Small(dw) => {
            let (lo, hi) = split_dword(dw);
            let mut buffer = Buffer::allocate(a.len() + b.len() + 1);
            buffer.push(lo);
            buffer.push(hi);
            buffer
        }
        Large(buffer) => buffer,
    };

    let (mut sa, mut sb) = ([0; 2], [0; 2]);
    let (mut wa, mut wb) = (magnitude_words(a, &mut sa), magnitude_words(b, &mut sb));
    if wa.len() < wb.len() {
        mem::swap(&mut wa, &mut wb);
    }
    let n = wa.len() + wb.len();
    if buffer.len() < n {
        // room for the product and one carry word, so that the push below
        // never reallocates
        buffer.ensure_capacity(n + 1);
        buffer.push_zeros(n - buffer.len());
    }

    // the accumulator's magnitude plus or minus the product's
    let relative = if sign == xsign { Positive } else { Negative };
    let mut allocation = MemoryAllocation::new(mul::memory_requirement_exact(n, wb.len()));
    let carry: SignedWord = {
        let (low, high) = buffer.split_at_mut(n);
        let c = mul::add_signed_mul(low, relative, wa, wb, &mut allocation.memory());
        add::add_signed_word_in_place(high, c)
    };
    drop(allocation);

    let result_sign = match carry {
        0 => xsign,
        1 => {
            buffer.push_resizing(1);
            xsign
        }
        _ => {
            // a borrow out of the top word: the product's magnitude exceeded
            // the accumulator's, and the buffer holds the difference modulo
            // a power of the word base
            debug_assert_eq!(carry, -1);
            negate_in_place(&mut buffer);
            -xsign
        }
    };
    Repr::from_buffer(buffer).with_sign(result_sign)
}

/// The sum of two signed representations (both small here).
fn add_signed_repr(x: Repr, y: Repr) -> Repr {
    (IBig(x) + IBig(y)).0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ptr(x: &IBig) -> *const Word {
        x.as_sign_words().1.as_ptr()
    }

    #[test]
    fn heap_accumulator_is_reused() {
        // an accumulator with room for the product and its carry keeps its
        // buffer: no temporary, no reallocation
        let mut x = IBig::from(UBig::from_words(&[1, 2, 3, 4, 5, 6, 7, 8]));
        let (a, b) = (IBig::from(UBig::from_words(&[9, 9, 9])), IBig::from(7));
        let before = ptr(&x);
        x.add_mul_assign(&a, &b);
        assert_eq!(ptr(&x), before);
        x.sub_mul_assign(&a, &b);
        assert_eq!(ptr(&x), before);
        assert_eq!(x, IBig::from(UBig::from_words(&[1, 2, 3, 4, 5, 6, 7, 8])));
    }

    #[test]
    fn negation_is_twos_complement() {
        let mut w = [1, 0, 0];
        negate_in_place(&mut w);
        assert_eq!(w, [Word::MAX, Word::MAX, Word::MAX]);
        let mut v = [0, 2];
        negate_in_place(&mut v);
        assert_eq!(v, [0, Word::MAX - 1]);
    }
}
