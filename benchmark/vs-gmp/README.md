# dashu-int against GMP, per operation, native and in the browser

Five integer operations on random numbers of $d$ decimal digits, $d = 10^3, 10^4, 10^5, 10^6$:
- `mul`: $a \cdot b$;
- `div`: $\lfloor n / b \rfloor$ with $n \approx a b$ ($2d$ by $d$ digits);
- `sqrt`: $\lfloor \sqrt{n} \rfloor$;
- `to_str` and `from_str`: decimal printing and parsing of $a$.

Each number is the best per-operation time over 3 batches of at least 0.2 s. Comparisons are like for like: native against native on the same CPU, and WebAssembly against WebAssembly in the same headless Chromium page.

Measured Nov 11 2026 on an AMD EPYC 7B13, one core per run.
- **dashu-int:** this fork's `integer/` at a0fc5358 (0.6.2).
- **GMP 6.3.0 with assembly:** Ubuntu's `libgmp.a`.
- **GMP 6.3.0, generic C:** built from the official tarball (sha256 `a3c2b80201b8...`) with `--disable-assembly`, `-O2`, 64-bit limbs. This shows what GMP's algorithms do without assembly.
- **GMP in the browser:** Pyodide 314.0.7's `gmpy2` 2.3.0 (GMP 6.3.0, 32-bit limbs).
- **"dashu-int + Sagebrush NTT":** Sagebrush's `sagebrush-bigint`, which is dashu-int 0.6.2 with its own NTT for large products. Measured earlier the same day, same machine and method.

## Native (times in ms)

| digits | op | GMP asm | GMP generic C | dashu-int | dashu / GMP asm | dashu / GMP generic | dashu-int + Sagebrush NTT |
|---:|---|---:|---:|---:|---:|---:|---:|
| $10^3$ | mul | 0.00154 | 0.00608 | 0.00201 | 1.31 | 0.33 | 0.00193 |
|  | div | 0.00257 | 0.00755 | 0.00442 | 1.72 | 0.59 | 0.00439 |
|  | sqrt | 0.0012 | 0.00298 | 0.00264 | 2.20 | 0.89 | 0.00262 |
|  | to_str | 0.00402 | 0.00731 | 0.00794 | 1.98 | 1.09 | 0.00944 |
|  | from_str | 0.00213 | 0.005 | 0.00159 | 0.75 | 0.32 | 0.0016 |
| $10^4$ | mul | 0.0436 | 0.153 | 0.0823 | 1.89 | 0.54 | 0.0826 |
|  | div | 0.128 | 0.437 | 0.168 | 1.31 | 0.38 | 0.17 |
|  | sqrt | 0.0544 | 0.166 | 0.115 | 2.11 | 0.69 | 0.114 |
|  | to_str | 0.104 | 0.286 | 0.242 | 2.34 | 0.85 | 0.242 |
|  | from_str | 0.0561 | 0.183 | 0.113 | 2.01 | 0.62 | 0.114 |
| $10^5$ | mul | 1.04 | 3.21 | 3.12 | 3.00 | 0.97 | 3.48 |
|  | div | 3.13 | 10.2 | 6.03 | 1.93 | 0.59 | 6.37 |
|  | sqrt | 1.62 | 5.94 | 4.24 | 2.61 | 0.71 | 4.38 |
|  | to_str | 3.36 | 11 | 7.5 | 2.23 | 0.68 | 7.67 |
|  | from_str | 1.55 | 5.65 | 4.06 | 2.63 | 0.72 | 4.26 |
| $10^6$ | mul | 14 | 46.7 | 46.3 | 3.32 | 0.99 | 30.2 |
|  | div | 44.6 | 142 | 178 | 4.00 | 1.25 | 195 |
|  | sqrt | 33.6 | 85.8 | 130 | 3.88 | 1.52 | 144 |
|  | to_str | 82.9 | 235 | 240 | 2.90 | 1.02 | 259 |
|  | from_str | 37.7 | 99.5 | 106 | 2.82 | 1.07 | 109 |

## WebAssembly in headless Chromium 149 (times in ms)

| digits | op | dashu-int (wasm) | dashu-int + Sagebrush NTT (wasm) | GMP via Pyodide gmpy2 | Pyodide GMP / dashu |
|---:|---|---:|---:|---:|---:|
| $10^3$ | mul | 0.00865 | 0.00865 | 0.11 | 12.7 |
|  | div | 0.0135 | 0.0141 | 0.203 | 15.0 |
|  | sqrt | 0.0111 | 0.0116 | 0.0657 | 5.9 |
|  | to_str | 0.0197 | 0.0202 | 0.133 | 6.7 |
|  | from_str | 0.00495 | 0.0051 | 0.1 | 20.2 |
| $10^4$ | mul | 0.285 | 0.312 | 2.95 | 10.4 |
|  | div | 0.624 | 0.623 | 7.84 | 12.6 |
|  | sqrt | 0.421 | 0.416 | 3.91 | 9.3 |
|  | to_str | 0.736 | 0.742 | 6.47 | 8.8 |
|  | from_str | 0.441 | 0.437 | 3.66 | 8.3 |
| $10^5$ | mul | 11.2 | 7.04 | 37.7 | 3.4 |
|  | div | 26.3 | 26.6 | 113 | 4.3 |
|  | sqrt | 17.2 | 17.5 | 102 | 5.9 |
|  | to_str | 25.9 | 26.5 | 208 | 8.1 |
|  | from_str | 15.2 | 15.6 | 96.1 | 6.3 |
| $10^6$ | mul | 112 | 66.9 | 717 | 6.4 |
|  | div | 545 | 562 | 1,902 | 3.5 |
|  | sqrt | 422 | 435 | 1,441 | 3.4 |
|  | to_str | 811 | 829 | 4,159 | 5.1 |
|  | from_str | 298 | 311 | 1,686 | 5.7 |

**Calibration: Pyodide's GMP is a poor build, not merely one without assembly.**
- In the same Pyodide, a 1,000-digit product takes 0.0113 ms with plain Python `int` and 0.111 ms with `gmpy2.mpz`. That is ten times slower than CPython's own big integers.
- Adding two 1,000-digit `mpz` takes 0.80 µs, against 0.075 µs natively.
- `calib.py` reproduces both numbers.

A carefully built generic-C GMP in wasm would do much better. Expect roughly the generic-C column times wasm's own slowdown, which is 2 to 4.5 times on these numbers. I did not build one, since there was no emscripten at hand.

## What dashu-int 0.6.2 has, from its source

- **Multiplication:**
  - schoolbook up to 24 words;
  - Karatsuba up to 96 words;
  - Toom-3 above that;
  - its own NTT above 4,000 words.

  There is no Toom-4, Toom-6.5 or Toom-8.5. GMP has all of those, with tuned thresholds, plus a Schönhage–Strassen FFT.
- **Division:** divide-and-conquer (Burnikel–Ziegler), which costs $O(M(n) \log n)$. GMP switches to Newton-based "mu" division, which costs $O(M(n))$. At $10^6$ digits, dashu's division costs 3.8 of its own multiplications; GMP's costs 3.2.
- **Square root:** Zimmermann's Karatsuba square root, on top of that division.
- **Decimal conversion:** divide-and-conquer on top of that division.

## Reading the numbers

- **Native, up to $10^5$ digits, the gap to GMP is mostly assembly.**
  - GMP's assembly build is 1.8 to 3.9 times faster than its own generic C.
  - dashu-int takes 0.32 to 0.97 of generic-C GMP's time, with one exception: decimal printing at $10^3$ digits (1.09).
- **At $10^6$ digits, algorithms take over.**
  - dashu-int's multiplication matches generic-C GMP (0.99).
  - Its division and square root are 1.25 and 1.52 times generic-C GMP's, from Burnikel–Ziegler and the square root built on it.
- **In the browser, where assembly isn't an option:** dashu-int is 3.4 to 20 times faster than the GMP that Pyodide ships.

**What would close the gap, in order of payoff:**
1. **Newton division and square root on top of the fastest multiplication**, with subquadratic radix conversion through them. This is the $10^6$ algorithmic gap, and it helps native and wasm alike.
2. **Toom-4 or Toom-6.5 and per-target thresholds**, between Toom-3 and the NTT, for $10^4$ to $10^5$ digits.
3. **The native small-size inner loops**, where GMP's assembly wins (below $10^4$ digits). Rust can reach the same instructions through `core::arch` intrinsics (`_mulx_u64`, `_addcarry_u64`) or inline `asm!`. In wasm, which has no 64x64->128 multiply, the lever is 128-bit SIMD.

**Caveats:**
- Single runs on a shared machine, so treat differences under 10% as noise.
- The two sides use different random numbers of the same sizes.
- Pyodide's timings go through Python's interpreter, about 0.06 µs a call, negligible here.
- The "well-built GMP in wasm" figure is an estimate.

## Reproducing

- **Native dashu:** `cd kernel && cargo run --release` prints the dashu-int rows.
- **Native GMP:** `gcc -O2 -o gmpbench gmpbench.c -lgmp && ./gmpbench`, linked against a GMP with assembly, or one configured with `--disable-assembly`. With GMP 6.3.0 and a gcc that defaults to C23, configure needs `CC="gcc -std=gnu17"`. `python3 gmp_bench.py` runs the same operations through gmpy2.
- **Browser:** `cd kernel && cargo build --release --lib --target wasm32-unknown-unknown`, copy `vs_gmp_kernel.wasm` here, then `node chrome-bench.mjs`. It needs a `chromium` on PATH (or `CHROME=...`) and network access to Pyodide's CDN.

`results/` holds the raw outputs these tables come from.
