//! Phase 0, part C: the engine's side of the boundary. Engine requests parse each big integer from a
//! decimal string and print results in decimal; this times dashu-int's decimal and hex conversions at
//! 1 word to about 10^5 digits. Prints `words op ns`.
use dashu_base::BitTest;
use dashu_int::UBig;
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
    let reps = ((0.2 / one).ceil() as u32).clamp(1, 5_000_000);
    let mut best = f64::MAX;
    for _ in 0..3 {
        let t = Instant::now();
        black_box(f(reps));
        best = best.min(t.elapsed().as_secs_f64() / reps as f64);
    }
    best * 1e9
}

fn main() {
    for &w in &[1usize, 4, 16, 64, 256, 1024, 5191] {
        let mut seed = 0x9e3779b97f4a7c15u64 ^ w as u64;
        let x = rnd(w, &mut seed);
        let dec = x.to_string();
        let hex = format!("{:x}", x);
        assert_eq!(UBig::from_str_radix(&dec, 10).unwrap(), x);
        assert_eq!(UBig::from_str_radix(&hex, 16).unwrap(), x);
        let rows = [
            ("to_dec", time(|r| (0..r).map(|_| black_box(x.to_string()).len() as u64).sum())),
            ("from_dec", time(|r| (0..r).map(|_| black_box(UBig::from_str_radix(&dec, 10).unwrap()).bit_len() as u64).sum())),
            ("to_hex", time(|r| (0..r).map(|_| black_box(format!("{:x}", x)).len() as u64).sum())),
            ("from_hex", time(|r| (0..r).map(|_| black_box(UBig::from_str_radix(&hex, 16).unwrap()).bit_len() as u64).sum())),
        ];
        for (op, ns) in rows {
            println!("{} {} {:.4e}", w, op, ns);
        }
    }
}
