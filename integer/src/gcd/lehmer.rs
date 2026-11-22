use alloc::alloc::Layout;
use core::borrow::BorrowMut as _;
use core::mem;
use dashu_base::{ExtendedGcd, Gcd};

use crate::{
    add,
    arch::word::{DoubleWord, SignedDoubleWord, SignedWord, Word},
    cmp::cmp_in_place,
    div,
    helper_macros::debug_assert_zero,
    memory::{self, Memory},
    mul,
    primitive::{
        extend_word, highest_dword, locate_top_word_plus_one, signed_extend_word, split_dword,
        split_signed_dword, WORD_BITS,
    },
    shift, Sign,
};

/// Remove the leading zero words in an owning reference. Return
/// a null slice if the input is zero (but the start pointer still
/// points to the same address as input).
#[inline]
fn trim_leading_zeros(words: &mut [Word]) -> &mut [Word] {
    words.split_at_mut(locate_top_word_plus_one(words)).0
}

/// Minimum length of words for using double word guessing on lehmer step
pub const MIN_DWORD_GUESS_LEN: usize = 300;

/// Estimate the bezout coefficients using the highest word,
/// return the coefficients (a, b, c, d) such that gcd(x, y) = gcd(ax - by, dy - cx)
///
/// If the guess has completely failed, then (b and c will be zero.)
fn lehmer_guess(mut xbar: Word, mut ybar: Word) -> (Word, Word, Word, Word) {
    debug_assert!(xbar >= ybar);
    const COEFF_LIMIT: Word = SignedWord::MAX as Word;

    let (mut a, mut b, mut c, mut d) = (1, 0, 0, 1);
    while ybar != 0 {
        let q = xbar / ybar;
        if q > COEFF_LIMIT {
            break;
        }

        let r = a + q * c;
        let s = b + q * d;
        let t = xbar - q * ybar;

        if r > COEFF_LIMIT || s > COEFF_LIMIT {
            break;
        }
        if t < s || t + r > ybar - c {
            break;
        }

        a = r;
        b = s;
        xbar = t;

        if xbar == b {
            break;
        }

        let q = ybar / xbar;
        if q > COEFF_LIMIT {
            break;
        }

        let r = d + q * b;
        let s = c + q * a;
        let t = ybar - q * xbar;

        if r > COEFF_LIMIT || s > COEFF_LIMIT {
            break;
        }
        if t < s || t + r > xbar - c {
            break;
        }

        d = r;
        c = s;
        ybar = t;

        if ybar == c {
            break;
        }
    }

    (a, b, c, d)
}

/// Get the (aligned) highest bits of x and y with the width of a Word.
/// If y < x, then y will be padded with leading zeros.
#[inline]
fn highest_word_normalized(x: &[Word], y: &[Word]) -> (Word, Word) {
    let x_hi2 = highest_dword(x);
    let y_hi2 = match x.len() - y.len() {
        0 => highest_dword(y),
        1 => extend_word(*y.last().unwrap()),
        _ => 0,
    };
    let shift = x_hi2.leading_zeros();
    let (_, x_hi) = split_dword(x_hi2 << shift);
    let (_, y_hi) = split_dword(y_hi2 << shift);
    (x_hi, y_hi)
}

/// Same as [lehmer_guess] but use the highest double word
fn lehmer_guess_dword(mut xbar: DoubleWord, mut ybar: DoubleWord) -> (Word, Word, Word, Word) {
    debug_assert!(xbar >= ybar);
    const COEFF_LIMIT: DoubleWord = SignedWord::MAX as DoubleWord;

    let (mut a, mut b, mut c, mut d) = (1, 0, 0, 1);
    while ybar != 0 {
        let q = xbar / ybar;

        if q > COEFF_LIMIT {
            break;
        }

        let r = a + q * c;
        let s = b + q * d;
        let t = xbar - q * ybar;

        if r > COEFF_LIMIT || s > COEFF_LIMIT {
            break;
        }
        if t < s || t + r > ybar - c {
            break;
        }

        a = r;
        b = s;
        xbar = t;

        if xbar == b {
            break;
        }

        let q = ybar / xbar;
        if q > COEFF_LIMIT {
            break;
        }

        let r = d + q * b;
        let s = c + q * a;
        let t = ybar - q * xbar;

        if r > COEFF_LIMIT || s > COEFF_LIMIT {
            break;
        }
        if t < s || t + r > xbar - c {
            break;
        }

        d = r;
        c = s;
        ybar = t;

        if ybar == c {
            break;
        }
    }

    // by now, abcd are all smaller than COEFF_LIMIT, so they fits in Words
    (a as Word, b as Word, c as Word, d as Word)
}

/// Get the (aligned) highest bits of x and y with the width of a DoubleWord.
/// If y < x, then y will be padded with leading zeros.
#[inline]
fn highest_dword_normalized(x: &[Word], y: &[Word]) -> (DoubleWord, DoubleWord) {
    debug_assert!(x.len() >= 3);
    let (x0, x_lo) = x.split_last().unwrap();
    let x12 = highest_dword(x_lo);
    let (y0, y12) = match x.len() - y.len() {
        0 => {
            let (y0, y_lo) = y.split_last().unwrap();
            (*y0, highest_dword(y_lo))
        }
        1 => (0, highest_dword(y)),
        2 => (0, extend_word(*y.last().unwrap())),
        _ => (0, 0),
    };
    let shift = x0.leading_zeros();
    let x_hi = extend_word(*x0) << (shift + WORD_BITS) | x12 >> (WORD_BITS - shift);
    let y_hi = extend_word(y0) << (shift + WORD_BITS) | y12 >> (WORD_BITS - shift);
    (x_hi, y_hi)
}

/// Calculate (x, y) = (a*x - b*y, d*y - c*x) in a single run.
///
/// Assuming x > y and (a, b, c, d) are calculated by lehmer_guess. Since the
/// coefficients are estimated by lehmer_guess, it will reduce the x by almost 1.
pub(crate) fn lehmer_step(x: &mut [Word], y: &mut [Word], a: Word, b: Word, c: Word, d: Word) {
    debug_assert!(x.len() >= y.len() && x.len() - y.len() <= 1);
    debug_assert!(a <= SignedWord::MAX as Word && b <= SignedWord::MAX as Word);
    debug_assert!(c <= SignedWord::MAX as Word && d <= SignedWord::MAX as Word);
    let (a, b) = (signed_extend_word(a), signed_extend_word(b));
    let (c, d) = (signed_extend_word(c), signed_extend_word(d));

    let (mut x_carry, mut y_carry) = (0, 0);
    for (x_i, y_i) in x.iter_mut().zip(y.iter_mut()) {
        let (sx_i, sy_i) = (signed_extend_word(*x_i), signed_extend_word(*y_i));
        let (x_new, cx) = split_signed_dword(a * sx_i - b * sy_i + x_carry as SignedDoubleWord);
        let (y_new, cy) = split_signed_dword(d * sy_i - c * sx_i + y_carry as SignedDoubleWord);
        x_carry = cx;
        y_carry = cy;
        *x_i = x_new;
        *y_i = y_new;
    }

    // if the carry words are not zero, then at most one additional step is required
    if x_carry != 0 {
        let x_top = x.last_mut().unwrap();
        debug_assert_eq!(y_carry as SignedDoubleWord, c * signed_extend_word(*x_top));
        let (x_new, cx) =
            split_signed_dword(a * signed_extend_word(*x_top) + x_carry as SignedDoubleWord);
        debug_assert_eq!(cx, 0);
        *x_top = x_new;
    }
}

/// Temporary memory required for gcd.
#[inline]
pub fn memory_requirement_up_to(lhs_len: usize, rhs_len: usize) -> Layout {
    // The scratchpad is reserved ONCE from the initial operand lengths, but each
    // euclidean step's division dispatches on the *current* lengths. As Lehmer shrinks
    // y while x stays large, a later step can divide a still-large x by a much smaller y
    // and take the Burnikel-Ziegler path — even when the first (similarly-sized)
    // division was "simple" and needed no scratch. `div::memory_requirement_exact` only
    // bounds a *single* division with the given lengths, so it under-reserves here.
    //
    // Reserve the worst case reachable in the loop instead: every in-loop division has a
    // divisor R ≤ rhs_len and a quotient split min(R/2, L-R) ≤ rhs_len/2, and
    // mul::memory_requirement_up_to is non-decreasing in both arguments, so
    // mul::memory_requirement_up_to(rhs_len, rhs_len/2) bounds them all (independent of
    // lhs_len). It returns zero_layout for small operands automatically, since mul's own
    // schoolbook threshold gates it.
    let _ = lhs_len;
    mul::memory_requirement_up_to(rhs_len, rhs_len / 2)
}

pub(crate) fn gcd_in_place(
    lhs: &mut [Word],
    rhs: &mut [Word],
    memory: &mut Memory,
) -> (usize, bool) {
    // keep x >= y though the algorithm, and track the source of x and y
    debug_assert!(cmp_in_place(lhs, rhs).is_ge());
    let (mut x, mut y, mut swapped) = (lhs, rhs, false);

    while y.len() > 2 {
        // Guess the coefficients based on the highest words
        let (a, b, c, d) = if x.len() < MIN_DWORD_GUESS_LEN {
            let (x_hi, y_hi) = highest_word_normalized(x, y);
            lehmer_guess(x_hi, y_hi)
        } else {
            let (x_hi, y_hi) = highest_dword_normalized(x, y);
            lehmer_guess_dword(x_hi, y_hi)
        };

        if b == 0 {
            // The guess has failed, do a euclidean step (x, y) = (y, x % y)
            let (shift, fast_div_top) = div::normalize(y);
            let _rem = div::div_rem_unshifted_in_place(x, y, shift, fast_div_top, memory);
            let mut r = &mut x[..y.len()];
            debug_assert_zero!(shift::shr_in_place(y, shift));
            debug_assert_zero!(shift::shr_in_place(r, shift));
            r = trim_leading_zeros(r);

            // swap: (x, y) = (y, r)
            x = mem::replace(&mut y, r);
            swapped = !swapped;
        } else {
            // The lehmer guess succeeded, use the coefficients to update x, y
            lehmer_step(x, y, a, b, c, d);
            x = trim_leading_zeros(x);
            y = trim_leading_zeros(y);
            if cmp_in_place(x, y).is_le() {
                mem::swap(&mut x, &mut y);
                swapped = !swapped;
            }
        }
    }

    if y.is_empty() {
        // the gcd result is in x
        (x.len(), swapped)
    } else if y.get(1).unwrap_or(&0) == &0 {
        // forward to single word gcd, store result in x
        let y_word = *y.first().unwrap();
        let x_word = div::rem_by_word(x, y_word);
        x[0] = x_word.gcd(y_word);
        (1, swapped)
    } else {
        // forward to double word gcd, store result in x
        let y_dword = highest_dword(y);
        let x_dword = div::rem_by_dword(x, y_dword);
        let (g_lo, g_hi) = split_dword(x_dword.gcd(y_dword));

        x[0] = g_lo;
        if g_hi != 0 {
            x[1] = g_hi;
            (2, swapped)
        } else {
            (1, swapped)
        }
    }
}

/// Calculate `(x, y) = (a*x + b*y, c*x + d*y)` in a single run, the argument `len` is used
/// to bound the iteration range (this function operates on x[..len] and y[..len]). Returns carry words.
fn lehmer_ext_step(
    x: &mut [Word],
    y: &mut [Word],
    len: usize,
    a: Word,
    b: Word,
    c: Word,
    d: Word,
) -> (Word, Word) {
    debug_assert!(len <= x.len() && len <= y.len());
    debug_assert!(a <= SignedWord::MAX as Word && b <= SignedWord::MAX as Word);
    debug_assert!(c <= SignedWord::MAX as Word && d <= SignedWord::MAX as Word);
    let (a, b) = (extend_word(a), extend_word(b));
    let (c, d) = (extend_word(c), extend_word(d));

    let (mut x_carry, mut y_carry) = (0, 0);
    for (x_i, y_i) in x.iter_mut().zip(y.iter_mut()).take(len) {
        let (sx_i, sy_i) = (extend_word(*x_i), extend_word(*y_i));
        let (x_new, cx) = split_dword(a * sx_i + b * sy_i + extend_word(x_carry));
        let (y_new, cy) = split_dword(c * sx_i + d * sy_i + extend_word(y_carry));
        x_carry = cx;
        y_carry = cy;
        *x_i = x_new;
        *y_i = y_new;
    }
    (x_carry, y_carry)
}

/// Temporary memory required for extended gcd.
pub fn memory_requirement_ext_up_to(lhs_len: usize, rhs_len: usize) -> Layout {
    // Required memory:
    // - two numbers (t0 & t1) with at most the same size as lhs, add 2 buffer words: the
    //   final reduction's update (t0 += x*t1, x not trimmed) spans up to lhs_len + 2 words
    //   (see gcd_ext_in_place); do not tighten this to 1
    // - temporary space for a division (for euclidean step, see memory_requirement_up_to
    //   for why the loop-wide bound rather than the initial lengths), and later a
    //   multiplication (for coeff update)
    let t_words = 2 * lhs_len + 4;
    memory::add_layout(
        memory::array_layout::<Word>(t_words),
        memory::max_layout(
            mul::memory_requirement_up_to(rhs_len, rhs_len / 2), // worst-case in-loop division
            mul::memory_requirement_up_to(lhs_len, lhs_len / 2), // for coeff update
        ),
    )
}

/// `t += (q_top * B^len(q_lo) + q_lo) * t1`, where B is the word base, for a `t` whose
/// words past the sum's length are zero (the carries propagate into them).
///
/// The caller passes `t` long enough for the sum: `len(q_lo) + len(t1) + 1` words and
/// `t`'s own length + 1, whichever is more. The sum is a Bezout coefficient of the inputs,
/// at most `lhs` in magnitude, so it always fits; a carry out of `t` would mean a wrong
/// coefficient, and is a release assertion.
fn add_mul_into(t: &mut [Word], q_lo: &[Word], q_top: Word, t1: &[Word], memory: &mut Memory) {
    let qt1_len = q_lo.len() + t1.len();
    assert!(qt1_len < t.len());
    let c = mul::add_signed_mul(&mut t[..qt1_len], Sign::Positive, q_lo, t1, memory);
    debug_assert!(c >= 0);
    if c > 0 {
        let overflow = add::add_word_in_place(&mut t[qt1_len..], c as Word);
        assert!(!overflow, "a Bezout coefficient overflowed its buffer");
    }
    if q_top > 0 {
        let carry = mul::add_mul_word_in_place(&mut t[q_lo.len()..], q_top, t1);
        assert!(carry == 0, "a Bezout coefficient overflowed its buffer");
    }
}

pub fn gcd_ext_in_place(
    lhs: &mut [Word],
    rhs: &mut [Word],
    memory: &mut Memory,
) -> (usize, usize, Sign) {
    let lhs_len = lhs.len();

    // keep x >= y though the algorithm, and track the source of x and y using the swapped flag
    debug_assert!(cmp_in_place(lhs, rhs).is_ge());
    // Use `borrow_mut` to be able to claim back `lhs` and `rhs`
    let (mut x, mut y) = (lhs.borrow_mut(), rhs.borrow_mut());
    let mut swapped = false;

    // the normal way is to have four variables s0, s1, t0, t1 and keep gcd(x, y) = gcd(lhs, rhs),
    // x = s0*lhs - t0*rhs, y = t1*rhs - s1*lhs. Here we simplify it by only tracking the
    // coefficient of rhs, so that x = -t0*rhs mod lhs, y = t1*rhs mod lhs,
    //
    // Both are zero-filled, and every update below recomputes the length with
    // `locate_top_word_plus_one` over the words it wrote, so the words past t0_len and
    // t1_len stay zero. Throughout, t1*x + t0*y = lhs with every term nonnegative (it holds
    // at the start, 1*lhs + 0*rhs; a euclidean step keeps it; a Lehmer step multiplies it by
    // a determinant-1 matrix; a swap exchanges both pairs), so t1 <= lhs/x, t0 <= lhs/y: at
    // most lhs_len words each. The 2 extra words are for the final reduction's span.
    let (mut t0, mut memory) = memory.allocate_slice_fill::<Word>(lhs_len + 2, 0);
    let (mut t1, mut memory) = memory.allocate_slice_fill::<Word>(lhs_len + 2, 0);
    let (mut t0_len, mut t1_len) = (1, 1);
    *t1.first_mut().unwrap() = 1;

    // loop, reduce x, y until the smaller one (y) fits in a single word
    while y.len() > 1 {
        // Guess the coefficients based on the highest words
        let (a, b, c, d) = if x.len() < MIN_DWORD_GUESS_LEN {
            let (x_hi, y_hi) = highest_word_normalized(x, y);
            lehmer_guess(x_hi, y_hi)
        } else {
            let (x_hi, y_hi) = highest_dword_normalized(x, y);
            lehmer_guess_dword(x_hi, y_hi)
        };

        if b == 0 {
            // The guess has failed, do a euclidean step (x, y) = (y, x % y)
            let (shift, fast_div_top) = div::normalize(y);
            let q_top = div::div_rem_unshifted_in_place(x, y, shift, fast_div_top, &mut memory);
            let (mut r, mut q_lo) = x.split_at_mut(y.len());
            debug_assert_zero!(shift::shr_in_place(y, shift));
            debug_assert_zero!(shift::shr_in_place(r, shift));
            r = trim_leading_zeros(r);
            if q_top == 0 {
                q_lo = trim_leading_zeros(q_lo);
            }

            // update coefficient t0 += q*t1, over all of t0's words: with a small quotient
            // t0 can be longer than q*t1, and its high words must be kept (and the carry
            // added into them, not written over them). By the invariant above, len(q_lo) +
            // t1_len <= (len(x) - len(y)) + (lhs_len - len(x) + 1) < lhs_len and t0_len <=
            // lhs_len, so `top` is within t0's lhs_len + 2 words.
            let qt1_len = q_lo.len() + t1_len;
            let top = qt1_len.max(t0_len) + 1;
            assert!(top <= t0.len());
            debug_assert!(t0[t0_len..top].iter().all(|w| *w == 0));
            add_mul_into(&mut t0[..top], q_lo, q_top, &t1[..t1_len], &mut memory);
            t0_len = locate_top_word_plus_one(&t0[..top]);

            // swap: (x, y) = (y, r)
            x = mem::replace(&mut y, r);
            mem::swap(&mut t0, &mut t1);
            mem::swap(&mut t0_len, &mut t1_len);
            swapped = !swapped;
        } else {
            // The lehmer guess succeeded, use the coefficients to update x, y and t0, t1
            lehmer_step(x, y, a, b, c, d);
            x = trim_leading_zeros(x);
            y = trim_leading_zeros(y);

            // (t0, t1) = (a*t0 + b*t1, c*t0 + d*t1): the coefficients' magnitudes, which
            // keep t1*x + t0*y = lhs (unsigned operations).
            let tmax_len = t0_len.max(t1_len);
            let (t0_carry, t1_carry) = lehmer_ext_step(t0, t1, tmax_len, a, b, c, d);
            if t0_carry > 0 {
                t0[tmax_len] = t0_carry;
                t0_len = tmax_len + 1;
            } else {
                t0_len = locate_top_word_plus_one(&t0[..tmax_len]);
            }
            if t1_carry > 0 {
                t1[tmax_len] = t1_carry;
                t1_len = tmax_len + 1;
            } else {
                t1_len = locate_top_word_plus_one(&t1[..tmax_len]);
            }

            // make sure x > y
            if cmp_in_place(x, y).is_le() {
                mem::swap(&mut x, &mut y);
                mem::swap(&mut t0, &mut t1);
                mem::swap(&mut t0_len, &mut t1_len);
                swapped = !swapped;
            }
        }
    }

    // If y is zero, then the gcd result is in x now.
    // Note that y.len() == 0 is equivalent to y == 0, which is guaranteed by trim_leading_zeros.
    if y.is_empty() {
        let x_len = x.len();
        // We are not using the borrwed `x` / `y` anymore, so we can
        // claim back the original `lhs` / `rhs`.
        if !swapped {
            rhs[..x_len].copy_from_slice(&lhs[..x_len]);
        }
        lhs[..t0_len].copy_from_slice(&t0[..t0_len]);
        let sign = if swapped {
            Sign::Positive
        } else {
            Sign::Negative
        };
        return (x_len, t0_len, sign);
    }

    // before forwarding to single word gcd, first reduce x by y:
    // x_word = x % y; x /= y
    let y_word = *y.first().unwrap();
    let x_word = div::div_by_word_in_place(x, y_word);
    // t0 += x*t1, over all of t0's words, as in the euclidean step (no input found reaches
    // it with t0 longer than x*t1, but the same form cannot truncate). x is not trimmed:
    // len(x) + t1_len <= lhs_len + 1, so `top` is at most lhs_len + 2, t0's length.
    let top = (x.len() + t1_len).max(t0_len) + 1;
    assert!(top <= t0.len());
    debug_assert!(t0[t0_len..top].iter().all(|w| *w == 0));
    add_mul_into(&mut t0[..top], x, 0, &t1[..t1_len], &mut memory);
    t0_len = locate_top_word_plus_one(&t0[..top]);

    // forward to single word gcd
    let (g_word, cx, cy) = x_word.gcd_ext(y_word);
    swapped ^= (cx < 0) || (cx == 0 && cy > 0);

    // let lhs stores |b| = |cx| * t0 + |cy| * t1
    // by now, number of words in |b| should be close to lhs

    // We are not using the borrwed `x` / `y` anymore, so we can
    // claim back the original `lhs` / `rhs`.
    *rhs.first_mut().unwrap() = g_word;
    lhs.fill(0);

    let (cx, cy) = (cx.unsigned_abs(), cy.unsigned_abs());
    debug_assert_zero!(mul::add_mul_word_in_place(lhs, cx, &t0[..t0_len]));
    debug_assert_zero!(mul::add_mul_word_in_place(lhs, cy, &t1[..t1_len]));
    let sign = if swapped {
        Sign::Positive
    } else {
        Sign::Negative
    };
    (1, locate_top_word_plus_one(lhs), sign)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryAllocation;

    /// A coefficient longer than q * t1: its high words are kept and the carries reach
    /// them (the euclidean step's update with a small quotient), with and without a top
    /// quotient word.
    #[test]
    fn add_mul_into_keeps_and_carries_into_the_high_words() {
        let mut allocation = MemoryAllocation::new(mul::memory_requirement_up_to(2, 1));
        let mut memory = allocation.memory();
        // t = 5 B^2 + (B - 1) B + (B - 1), q = 1 (q_lo empty, q_top 1), t1 = 1
        let mut t = [Word::MAX, Word::MAX, 5, 0];
        add_mul_into(&mut t, &[], 1, &[1], &mut memory);
        assert_eq!(t, [0, 0, 6, 0]);
        // q_lo = [2], t1 = [B - 1]: t += 2 (B - 1)
        let mut t = [Word::MAX, 0, 7, 0];
        add_mul_into(&mut t, &[2], 0, &[Word::MAX], &mut memory);
        assert_eq!(t, [Word::MAX - 2, 2, 7, 0]);
        // a carry out of the product's words, into the high word: t += 1 (B - 1)
        let mut t = [Word::MAX, Word::MAX, 5, 0];
        add_mul_into(&mut t, &[1], 0, &[Word::MAX], &mut memory);
        assert_eq!(t, [Word::MAX - 1, 0, 6, 0]);
    }
}
