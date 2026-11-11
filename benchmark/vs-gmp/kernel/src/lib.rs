//! The measured operations, on dashu-int, shared by the native binary and the wasm32 module
//! (exports `setup(digits)` and `op(kind, reps)`, timed by the host).
use dashu_base::{BitTest, SquareRoot, UnsignedAbs};
use dashu_int::IBig;
use std::cell::RefCell;
use std::hint::black_box;
use std::str::FromStr;

struct St { a: IBig, b: IBig, n2: IBig, n2u: dashu_int::UBig, s: String }
thread_local!(static ST: RefCell<Option<St>> = const { RefCell::new(None) });

/// A random integer of exactly `bytes` bytes (top bit set), from a xorshift stream.
fn rnd(bytes: usize, seed: &mut u64) -> IBig {
    let mut v: Vec<u8> = (0..bytes).map(|_| { *seed ^= *seed << 13; *seed ^= *seed >> 7; *seed ^= *seed << 17; *seed as u8 }).collect();
    *v.last_mut().unwrap() |= 0x80;
    IBig::from(dashu_int::UBig::from_le_bytes(&v))
}

/// Operands of `digits` decimal digits: a, b; n2 = a*b + 12345 (2*digits); s = a in decimal.
#[no_mangle]
pub extern "C" fn setup(digits: u32) {
    let bytes = ((digits as f64) * 3.321928094887362 / 64.0).ceil() as usize * 8;
    let mut seed = 0x9e3779b97f4a7c15u64 ^ digits as u64;
    let a = rnd(bytes, &mut seed);
    let b = rnd(bytes, &mut seed);
    let n2 = &a * &b + IBig::from(12345);
    let s = a.to_string();
    let n2u = n2.clone().unsigned_abs();
    ST.with(|c| *c.borrow_mut() = Some(St { a, b, n2, n2u, s }));
}

/// Runs operation `k` `reps` times: 0 a*b, 1 n2 / b, 2 isqrt(n2), 3 decimal string of a, 4 parse decimal s.
#[no_mangle]
pub extern "C" fn op(k: u32, reps: u32) -> u32 {
    ST.with(|c| {
        let st = c.borrow();
        let st = st.as_ref().unwrap();
        let mut acc = 0u32;
        for _ in 0..reps {
            acc = acc.wrapping_add(match k {
                0 => black_box(&st.a * &st.b).bit_len() as u32,
                1 => black_box(&st.n2 / &st.b).bit_len() as u32,
                2 => black_box(st.n2u.sqrt()).bit_len() as u32,
                3 => black_box(st.a.to_string()).len() as u32,
                _ => black_box(IBig::from_str(&st.s).unwrap()).bit_len() as u32,
            });
        }
        acc
    })
}
