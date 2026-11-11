//! Phase 0, part A (native): dashu-int at the sizes Sagebrush's exact algorithms use most, 1 to 128 words.
//! Prints `words op ns` lines; gmpbench-small.c prints the same for GMP.
//!
//! Operands are preconstructed random numbers of exactly `words` 64-bit words (top bit set):
//! - mul: a*b;
//! - addmul: acc += a*b, into one accumulator;
//! - div1: a / w for a one-word w;
//! - div: n / b, with n = a*b + 12345 (2*words by words);
//! - gcd, xgcd: gcd(a, b), and with Bezout coefficients;
//! - powmod: x^e mod m, with x and m of `words` words and a 64-bit exponent e.
//! Each figure is the best of 3 batches, each sized from one timed run to take about 0.1 s.
use dashu_base::{BitTest, ExtendedGcd, Gcd};
use dashu_int::{fast_div::ConstDivisor, IBig, UBig};
use std::hint::black_box;
use std::time::Instant;

fn rnd(words: usize, seed: &mut u64) -> UBig {
    let mut v: Vec<u8> = (0..words * 8)
        .map(|_| {
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            *seed as u8
        })
        .collect();
    *v.last_mut().unwrap() |= 0x80;
    UBig::from_le_bytes(&v)
}

fn time(mut f: impl FnMut(u32) -> u64) -> f64 {
    let t = Instant::now();
    black_box(f(1));
    let one = t.elapsed().as_secs_f64().max(1e-8);
    let reps = ((0.1 / one).ceil() as u32).clamp(1, 50_000_000);
    let mut best = f64::MAX;
    for _ in 0..3 {
        let t = Instant::now();
        black_box(f(reps));
        best = best.min(t.elapsed().as_secs_f64() / reps as f64);
    }
    best * 1e9
}

fn main() {
    for &words in &[1usize, 2, 4, 8, 16, 32, 64, 128] {
        let mut seed = 0x9e3779b97f4a7c15u64 ^ words as u64;
        let a = rnd(words, &mut seed);
        let b = rnd(words, &mut seed);
        let w = (rnd(1, &mut seed).to_le_bytes()[0] as u64) << 56 | 12345;
        let n = &a * &b + UBig::from(12345u32);
        let (ai, bi) = (IBig::from(a.clone()), IBig::from(b.clone()));
        let m = rnd(words, &mut seed) | UBig::ONE; // an odd modulus
        let x = rnd(words, &mut seed) % &m;
        let e = rnd(1, &mut seed);
        let ring = ConstDivisor::new(m.clone());
        let xr = ring.reduce(x.clone());
        let rows: Vec<(&str, f64)> = vec![
            ("mul", time(|r| (0..r).map(|_| black_box(&a * &b).bit_len() as u64).sum())),
            ("addmul", {
                let mut acc = IBig::ZERO;
                time(|r| {
                    for _ in 0..r {
                        acc += &ai * &bi;
                    }
                    acc.bit_len() as u64
                })
            }),
            ("div1", time(|r| (0..r).map(|_| black_box(&a / w).bit_len() as u64).sum())),
            ("div", time(|r| (0..r).map(|_| black_box(&n / &b).bit_len() as u64).sum())),
            ("gcd", time(|r| (0..r).map(|_| black_box((&a).gcd(&b)).bit_len() as u64).sum())),
            ("xgcd", time(|r| (0..r).map(|_| black_box((&a).gcd_ext(&b)).0.bit_len() as u64).sum())),
            ("powmod", time(|r| (0..r).map(|_| black_box(xr.pow(&e)).residue().bit_len() as u64).sum())),
        ];
        // (correctness, against identities; the GMP side checks its own)
        assert_eq!(&(&n / &b) * &b + (&n % &b), n);
        let (g, s, t) = (&a).gcd_ext(&b);
        assert_eq!(&s * &ai + &t * &bi, IBig::from(g));
        for (op, ns) in rows {
            println!("{} {} {:.4e}", words, op, ns);
        }
    }
}
