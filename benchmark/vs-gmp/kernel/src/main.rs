//! Native timing: best per-op time over 3 batches of >= 0.2 s, at 10^3 .. 10^6 decimal digits.
use std::time::Instant;
fn main() {
    let names = ["mul", "div", "sqrt", "to_str", "from_str"];
    for d in [1_000u32, 10_000, 100_000, 1_000_000] {
        vs_gmp_kernel::setup(d);
        for k in 0..5u32 {
            let t = Instant::now(); vs_gmp_kernel::op(k, 1); let one = t.elapsed().as_secs_f64().max(1e-7);
            let reps = ((0.2 / one).ceil() as u32).clamp(1, 1_000_000);
            let mut best = f64::MAX;
            for _ in 0..3 { let t = Instant::now(); vs_gmp_kernel::op(k, reps); best = best.min(t.elapsed().as_secs_f64() / reps as f64); }
            println!("{} {} {:.3e}", d, names[k as usize], best);
        }
    }
}
