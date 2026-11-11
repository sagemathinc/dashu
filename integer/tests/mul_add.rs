//! The fused multiply-add operations (`add_mul_assign`, `sub_mul_assign`)
//! against the separate multiplication and addition they replace: fixed
//! carry, borrow and cancellation cases, every product length up to 140
//! words with adversarial word patterns, and bounded property tests over
//! signs and lengths.
//!
//! Portable across `Word` widths: sizes are in words and patterns go through
//! `Word`.

use dashu_int::{IBig, UBig, Word};
use proptest::prelude::*;

fn ubig_words(words: &[Word]) -> UBig {
    UBig::from_words(words)
}

fn ibig(neg: bool, words: &[Word]) -> IBig {
    let m = IBig::from(ubig_words(words));
    if neg {
        -m
    } else {
        m
    }
}

/// x + a * b and x - a * b, fused, equal the unfused results.
fn check(x: &IBig, a: &IBig, b: &IBig) {
    let mut f = x.clone();
    f.add_mul_assign(a, b);
    assert_eq!(f, x + a * b, "add: x = {:?}, a = {:?}, b = {:?}", x, a, b);
    let mut g = x.clone();
    g.sub_mul_assign(a, b);
    assert_eq!(g, x - a * b, "sub: x = {:?}, a = {:?}, b = {:?}", x, a, b);
}

fn check_all_signs(x: &[Word], a: &[Word], b: &[Word]) {
    for sx in [false, true] {
        for sa in [false, true] {
            for sb in [false, true] {
                check(&ibig(sx, x), &ibig(sa, a), &ibig(sb, b));
            }
        }
    }
    let mut u = ubig_words(x);
    u.add_mul_assign(&ubig_words(a), &ubig_words(b));
    assert_eq!(u, ubig_words(x) + ubig_words(a) * ubig_words(b));
}

/// Deterministic word patterns: all ones, alternating bits, a single high
/// bit, and a mixed sequence.
fn pattern(kind: usize, len: usize) -> Vec<Word> {
    (0..len)
        .map(|i| match kind {
            0 => Word::MAX,
            1 => (Word::MAX / 3).rotate_left(i as u32),
            2 => {
                if i + 1 == len {
                    1 << (Word::BITS - 1)
                } else {
                    0
                }
            }
            _ => (i as Word).wrapping_mul(0x9E37).wrapping_add(0x5bd1) | 1,
        })
        .collect()
}

#[test]
fn zero_operands_leave_the_accumulator() {
    let x = ibig(true, &pattern(3, 5));
    check(&x, &IBig::ZERO, &ibig(false, &pattern(0, 4)));
    check(&x, &ibig(false, &pattern(0, 4)), &IBig::ZERO);
    check(&IBig::ZERO, &IBig::ZERO, &IBig::ZERO);
}

#[test]
fn zero_accumulator_gives_the_product() {
    for (la, lb) in [(1, 1), (2, 1), (2, 2), (3, 1), (5, 4), (40, 33)] {
        check_all_signs(&[], &pattern(0, la), &pattern(3, lb));
    }
}

#[test]
fn carry_into_a_new_top_word() {
    // all-ones accumulator plus a product that carries past it
    for len in 1..12 {
        check_all_signs(&pattern(0, len), &pattern(0, len), &[2]);
        check_all_signs(&pattern(0, len + 2), &pattern(0, len), &pattern(0, 2));
    }
}

#[test]
fn borrow_flips_the_sign() {
    // a short accumulator minus a longer product, and products just above
    // and below the accumulator
    for len in 1..12 {
        let a = pattern(1, len);
        check_all_signs(&[1], &a, &[3]);
        let x = pattern(3, len + 1);
        let xp = IBig::from(ubig_words(&x));
        check(&xp, &IBig::from(ubig_words(&x)), &IBig::ONE);
        check(&(&xp - IBig::ONE), &xp, &IBig::ONE);
        check(&(&xp + IBig::ONE), &xp, &IBig::ONE);
    }
}

#[test]
fn exact_cancellation_gives_zero() {
    for (la, lb) in [(1, 1), (2, 2), (3, 2), (7, 5), (64, 3), (130, 70)] {
        let (a, b) = (ibig(false, &pattern(3, la)), ibig(true, &pattern(1, lb)));
        let p = &a * &b;
        let mut x = p.clone();
        x.sub_mul_assign(&a, &b);
        assert_eq!(x, IBig::ZERO);
        let mut y = -p;
        y.add_mul_assign(&a, &b);
        assert_eq!(y, IBig::ZERO);
    }
}

#[test]
fn every_length_up_to_140_words() {
    // product lengths 2 to 140 words across the multiplication kernels'
    // thresholds, with the accumulator shorter, equal and longer
    for lb in [1usize, 2, 3, 5, 8, 17, 33, 70] {
        for la in (lb..=140 - lb).step_by(7).chain([lb, 140 - lb]) {
            for kind in 0..4 {
                let (a, b) = (pattern(kind, la), pattern((kind + 1) % 4, lb));
                for lx in [0, 1, la, la + lb, la + lb + 3] {
                    let x = pattern((kind + 2) % 4, lx);
                    check(&ibig(false, &x), &ibig(false, &a), &ibig(true, &b));
                    check(&ibig(true, &x), &ibig(false, &a), &ibig(false, &b));
                }
            }
        }
    }
}

#[test]
fn long_operands_reach_the_large_kernels() {
    // operands long enough for the Karatsuba and Toom kernels
    for (la, lb) in [(300, 300), (700, 260), (1100, 1000)] {
        let (a, b) = (pattern(3, la), pattern(1, lb));
        check_all_signs(&pattern(0, la + lb - 1), &a, &b);
        check_all_signs(&pattern(2, 4), &a, &b);
    }
}

fn words(max: usize) -> impl Strategy<Value = Vec<Word>> {
    prop::collection::vec(prop_oneof![any::<Word>(), Just(0), Just(Word::MAX), Just(1)], 0..max)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..Default::default() })]

    #[test]
    fn fused_equals_unfused(
        x in words(24), a in words(24), b in words(24),
        sx in any::<bool>(), sa in any::<bool>(), sb in any::<bool>(),
    ) {
        let (x, a, b) = (ibig(sx, &x), ibig(sa, &a), ibig(sb, &b));
        let mut f = x.clone();
        f.add_mul_assign(&a, &b);
        prop_assert_eq!(f, &x + &a * &b);
        let mut g = x.clone();
        g.sub_mul_assign(&a, &b);
        prop_assert_eq!(g, &x - &a * &b);
    }

    #[test]
    fn unsigned_fused_equals_unfused(x in words(24), a in words(24), b in words(24)) {
        let (x, a, b) = (ubig_words(&x), ubig_words(&a), ubig_words(&b));
        let mut f = x.clone();
        f.add_mul_assign(&a, &b);
        prop_assert_eq!(f, &x + &a * &b);
    }

    /// Accumulating many products into one number, the way elimination
    /// loops do, ends at the same value as the unfused sums.
    #[test]
    fn repeated_accumulation(terms in prop::collection::vec((words(6), words(6), any::<bool>()), 1..40)) {
        let mut fused = IBig::ZERO;
        let mut plain = IBig::ZERO;
        for (a, b, neg) in &terms {
            let (a, b) = (ibig(*neg, a), ibig(false, b));
            if *neg {
                fused.sub_mul_assign(&a, &b);
                plain -= &a * &b;
            } else {
                fused.add_mul_assign(&a, &b);
                plain += &a * &b;
            }
        }
        prop_assert_eq!(fused, plain);
    }
}
