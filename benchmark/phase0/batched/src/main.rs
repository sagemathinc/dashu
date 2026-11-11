//! Phase 0, part B (native): the batched kernels of Sagebrush's multimodular algorithms, through
//! Sagebrush's own code (sagebrush-arith at a pinned master commit). Prints `kernel k words ns_per_item`;
//! gmpbench-batched.c prints the same for plain GMP loops.
//!
//! For k primes just below 2^62 (k = 10, 100, 1000), on N = 1000 signed integers of `words` 64-bit words,
//! where `words` fills about half the product of the primes (so every integer reconstructs):
//! - residues: x mod p for every prime (MultiCrt::residues), per integer;
//! - residues_pw: the same with precomputed word powers (MultiCrt::residues_with);
//! - crt: the integer back from its k residues (MultiCrt::reconstruct), per integer;
//! - dot: a dot product of length 100 over Z (sum of a_i b_i), per product term;
//! - ratrecon: a/b from a b^-1 mod P with |a|, b below about sqrt(P)/2 (zmat::ratrecon), per call.
//! Each figure is the best of 3 batches, each sized from one timed run to take about 0.2 s.
use sagebrush_arith::crt::MultiCrt;
use sagebrush_arith::zmat::ratrecon;
use sagebrush_bigint::nmod::prev_prime;
use sagebrush_bigint::{BigInt, BigUint, Sign};
use std::hint::black_box;
use std::time::Instant;

/// A random integer of exactly 64 words - 1 bits (bit 62 of the top word set), with the given sign.
fn rnd(words: usize, seed: &mut u64, neg: bool) -> BigInt {
    let mut v: Vec<u8> = (0..words * 8)
        .map(|_| {
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            *seed as u8
        })
        .collect();
    let top = v.last_mut().unwrap();
    *top = (*top & 0x7f) | 0x40;
    BigInt::from_biguint(if neg { Sign::Minus } else { Sign::Plus }, BigUint::from_bytes_le(&v))
}

/// b^-1 mod m, by the extended Euclidean algorithm (gcd(b, m) = 1).
fn inverse(b: &BigInt, m: &BigInt) -> BigInt {
    let zero = BigInt::from(0);
    let (mut r0, mut r1) = (m.clone(), ((b % m) + m) % m);
    let (mut t0, mut t1) = (zero.clone(), BigInt::from(1));
    while r1 != zero {
        let q = &r0 / &r1;
        let r2 = &r0 - &q * &r1;
        r0 = std::mem::replace(&mut r1, r2);
        let t2 = &t0 - &q * &t1;
        t0 = std::mem::replace(&mut t1, t2);
    }
    assert_eq!(r0, BigInt::from(1), "not invertible");
    ((t0 % m) + m) % m
}

fn time(mut f: impl FnMut(u32) -> u64, per: f64) -> f64 {
    let t = Instant::now();
    black_box(f(1));
    let one = t.elapsed().as_secs_f64().max(1e-7);
    let reps = ((0.2 / one).ceil() as u32).clamp(1, 1_000_000);
    let mut best = f64::MAX;
    for _ in 0..3 {
        let t = Instant::now();
        black_box(f(reps));
        best = best.min(t.elapsed().as_secs_f64() / reps as f64);
    }
    best * 1e9 / per
}

fn main() {
    const N: usize = 1000;
    for &k in &[10usize, 100, 1000] {
        let mut primes = Vec::with_capacity(k);
        let mut p = 1u64 << 62;
        for _ in 0..k {
            p = prev_prime(p);
            primes.push(p);
        }
        let crt = MultiCrt::new(&primes);
        let words = ((k * 62) / 64 / 2).max(1);
        let mut seed = 0x9e3779b97f4a7c15u64 ^ k as u64;
        let xs: Vec<BigInt> = (0..N).map(|i| rnd(words, &mut seed, i % 2 == 1)).collect();
        let pw = crt.word_powers(words);
        let rs: Vec<Vec<u64>> = xs.iter().map(|x| crt.residues(x)).collect();
        for (x, r) in xs.iter().zip(&rs) {
            assert_eq!(&crt.reconstruct(r), x, "CRT round trip");
            assert_eq!(&crt.residues_with(x, &pw), r, "residues_with");
        }
        let a: Vec<BigInt> = (0..100).map(|i| rnd(words, &mut seed, i % 3 == 0)).collect();
        let b: Vec<BigInt> = (0..100).map(|i| rnd(words, &mut seed, i % 5 == 0)).collect();
        // ratrecon: |num|, den < 2^(31 k - 2), and P > 2^(61.9 k) > 2 nb db
        let hw = ((k * 31 - 2) / 64).max(1);
        let bound = BigInt::from(1) << (64 * hw);
        let m = primes.iter().fold(BigInt::from(1), |acc, &p| acc * BigInt::from(p));
        let num = rnd(hw, &mut seed, true);
        let den = rnd(hw, &mut seed, false);
        let u = (((&num % &m) + &m) * inverse(&den, &m)) % &m;
        let (rn, rd) = ratrecon(&u, &m, &bound, &bound).expect("ratrecon finds it");
        assert_eq!(&rn * &den, &num * &rd, "ratrecon: the same fraction");
        let rows = [
            ("residues", time(|r| (0..r).map(|_| xs.iter().map(|x| black_box(crt.residues(x))[0]).fold(0, u64::wrapping_add)).sum(), N as f64)),
            ("residues_pw", time(|r| (0..r).map(|_| xs.iter().map(|x| black_box(crt.residues_with(x, &pw))[0]).fold(0, u64::wrapping_add)).sum(), N as f64)),
            ("crt", time(|r| (0..r).map(|_| rs.iter().map(|v| black_box(crt.reconstruct(v)).bits()).sum::<u64>()).sum(), N as f64)),
            ("dot", time(|r| (0..r).map(|_| {
                let mut s = BigInt::from(0);
                for (x, y) in a.iter().zip(&b) {
                    s += x * y;
                }
                black_box(s).bits()
            }).sum(), 100.0)),
            ("ratrecon", time(|r| (0..r).map(|_| black_box(ratrecon(&u, &m, &bound, &bound)).is_some() as u64).sum(), 1.0)),
        ];
        for (name, ns) in rows {
            println!("{} {} {} {:.4e}", name, k, words, ns);
        }
    }
}
