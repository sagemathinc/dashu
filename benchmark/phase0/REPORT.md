# Phase 0 report: what Sagebrush's integer arithmetic costs, and where

Phase 0 of Sagebrush's big-integer plan (`design/bigint-plan.md`, item W0). It is measurement only. The method is in `README.md`, the raw outputs in `results/` (first runs on a shared machine) and `results/vm4/` (clean runs).

**Clean runs.** All figures below come from one quiet VM: an AMD EPYC 7B13 pinned to one core (core 3), Nov 11 2026, 07:35-07:44Z. Sagebrush was at master `4d4225f9`; the integer code is dashu-int 0.6.2 under `sagebrush-bigint`. The first runs on the shared project machine agree with these within noise.

## The answer, in short

- **Integer arithmetic is a large share of real exact workloads' native time.**
  - F4 over $\mathbb{Q}$ spends **55 to 61%** of its single-thread time in big-integer arithmetic, plus 13 to 16% in allocation and copying.
  - A class group computation in a quintic number field spends **66%** there.
  - By the plan's stop rule, that is a reason to go on past Phase 2.
- **The cost is at small and medium sizes, in the multimodular steps, not in huge numbers.** In F4 the top items are:
  - gcds, for normalizing rationals;
  - the CRT lift;
  - building integers from limb buffers;
  - remainders by one word (the reductions mod each prime);
  - `malloc`/`free`;
  - one-word-by-many multiplications.
  
  No profiled workload spent visible time in numbers of $10^4$ digits or more.
- **ECM has its own integer arithmetic.** Factoring spends 99% of its time in `sagebrush_classgroup::nf::ecm::Mont`, a fixed-size Montgomery multiplication written inside the class-group crate. It is integer arithmetic outside the shared crate. The shared-crate rule says it belongs in `sagebrush-bigint`, as the reusable-inverse context of plan item W4b.
- **Next to GMP, at the sizes that matter** (1 to 128 words):
  - dashu is 1.2 to 3.5 times slower than GMP with assembly on multiplication, multiply-and-add and modular powers at every size, and on division from 4 words up (2n/n division also at 2 words). It is faster only on division at the smallest sizes: by one word at 1 to 2 words, and 2n/n at 1 word;
  - it is about even on gcd;
  - from 8 words up it is faster than GMP without assembly, or about even, on every operation.
  
  Sagebrush's multimodular kernels:
  - residues with precomputed powers are at GMP-with-assembly speed (0.90 to 1.10); without them they are 1.1 to 3.3 times slower, though the GMP side uses a simpler method (see Part B);
  - CRT reconstruction is 1.6 to 2.6 times slower;
  - dot products are 1.4 to 3.2 times slower;
  - rational reconstruction is 2.4 to 3.6 times slower, and slower even than GMP without assembly.
- **The Python-to-engine boundary costs 20 to 40 times more than it needs to at large sizes.** Big integers cross it as decimal strings in JSON; hex would do the same job far faster.

## Part D: where real workloads spend their time

Native, one thread (`SAGEBRUSH_THREADS=1`), pinned to one core, profiled with `perf` at 997 Hz (self time). Each workload ran through the native Python package built from master `4d4225f9` with line tables. `profile/classify.py` sorts the samples into kinds of work.

| workload | wall | integer arithmetic | allocation and copying | word-size modular arithmetic | the algorithm's own code |
|---|---:|---:|---:|---:|---:|
| F4 over $\mathbb{Q}$, katsura-8 | 12.2 s | **60.6%** | 16.0% | 4.8% | 16.0% |
| F4 over $\mathbb{Q}$, cyclic-7 | 8.4 s | **55.0%** | 13.3% | 12.3% | 16.7% |
| class group, $x^5 - 3x^4 + 7x - 1234567$ | 6.9 s | **65.9%** | 8.1% | 0 | 24.4% |
| ECM, three primes near $10^{23}$, $10^{25}$, $10^{27}$ | 14.9 s | 0 in dashu; **99%** in ECM's own Montgomery arithmetic | 0 | 0 | 99.8% |

In F4 over $\mathbb{Q}$ (katsura-8), the integer share splits as:
- representation, addition and conversion: 19%;
- division and remainder: 18%, mostly remainders by one word, that is, residues mod each prime;
- gcd: 12%;
- multiplication: 12%, mostly one word by many.

Its top functions:

| share | function |
|---:|---|
| 7.8% | `BigInt::gcd` |
| 7.8% | `f4q::Crt::lift` |
| 6.9% | `Repr::from_buffer` |
| 6.0% | `div::rem_by_word` |
| 4.7% | `malloc` |
| 4.3% | `mul_ibig` |
| 4.3% | `mul_large_dword` |
| 3.3% | `_int_free` |
| 3.1% | `cfree` |

The class group's top functions:

| share | function |
|---:|---|
| 15.3% | `add_mul_dword_same_len_in_place_bmi2` (schoolbook multiplication's inner loop) |
| 8.2% | `linalg::eliminate_tracked` |
| 6.4% | `div_rem_highest_word` |
| 5.2% | squaring |
| 4.8% | `sub_mul_dword_same_len_in_place_bmi2` |

**Not captured** (to redo):
- **Modular symbols** (level 10007, $T_2$'s characteristic polynomial, 16.4 s). `perf record -p` attached to the process and recorded only 0.02 s. The engine's work probably ran on threads started after it attached. The next attempt should start perf at launch, with the workload under the same user.
- **The imaginary quadratic class group** (1.45 s). Only a partial profile, 0.013 MB. What there is shows 84% in its own multimodular linear algebra (`det_crt_impl`, `hnf_mod_until`).
- **mpmath at high precision.** The native package ran CPython's own mpmath on Python integers (the VM has no gmpy2), not Sagebrush's multiprecision core, so the 64 s run says nothing about our integers. The core needs its own harness.
- **Operand sizes.** The size histogram from an instrumented facade build is not done yet.

## Part A: kernels at 1 to 128 words

Times are ns per operation, as the ratio dashu ÷ GMP 6.3.0 with assembly. In parentheses: ÷ GMP built as generic C. Below 1 means dashu is faster. Files: `results/vm4/A-*.txt`.

| words | mul | addmul | div by 1 word | div $2n/n$ | gcd | xgcd | powmod |
|---:|---|---|---|---|---|---|---|
| 1 | 1.23 (0.94) | 2.51 (2.21) | 0.60 (0.70) | 0.55 (0.47) | 0.63 (0.40) | 0.99 (0.88) | 1.67 (0.79) |
| 2 | 2.68 (0.98) | 2.56 (1.52) | 0.52 (0.47) | 2.26 (1.39) | 1.03 (0.78) | 1.37 (1.20) | 1.89 (0.47) |
| 4 | 3.31 (1.11) | 3.45 (1.28) | 1.93 (1.21) | 2.78 (1.62) | 0.84 (0.70) | 0.91 (0.70) | 3.24 (0.88) |
| 8 | 1.94 (0.50) | 2.19 (0.62) | 1.91 (1.03) | 2.19 (1.04) | 0.81 (0.64) | 0.96 (0.67) | 2.34 (0.64) |
| 16 | 1.53 (0.38) | 1.60 (0.42) | 2.04 (1.09) | 1.89 (0.71) | 0.87 (0.58) | 1.05 (0.56) | 2.08 (0.53) |
| 32 | 1.58 (0.40) | 1.56 (0.40) | 1.79 (0.91) | 1.75 (0.55) | 0.94 (0.54) | 1.21 (0.55) | 1.99 (0.52) |
| 64 | 1.68 (0.43) | 1.64 (0.42) | 1.67 (0.92) | 1.78 (0.60) | 1.05 (0.54) | 1.44 (0.54) | 1.86 (0.49) |
| 128 | 1.60 (0.41) | 1.63 (0.42) | 1.76 (0.86) | 1.31 (0.38) | 1.20 (0.49) | 1.60 (0.52) | 2.15 (0.53) |

At 2 to 4 words, multiplication is 2.7 to 3.3 times GMP with assembly. That is about the same as generic GMP, so it is per-operation overhead (allocation, dispatch), not the multiply loop. From 8 words up the gap to GMP with assembly is about 1.5 to 2 times, which is the assembly.

## Part B: Sagebrush's multimodular kernels

Sagebrush's own code (`MultiCrt`, `zmat::ratrecon` at `4d4225f9`) against the same algorithms as plain GMP loops. The inputs are 1,000 integers filling half the product of $k$ primes just below $2^{62}$. Ratios are Sagebrush ÷ GMP with assembly; in parentheses, ÷ generic C. Files: `results/vm4/B-*.txt`.

| $k$ (words) | residues | residues, precomputed powers | CRT | dot product | rational reconstruction |
|---|---|---|---|---|---|
| 10 (4) | 1.12 (0.61) | 0.90 (0.49) | 2.57 (1.18) | 3.15 (1.25) | 2.41 (1.62) |
| 100 (48) | 1.98 (0.31) | 1.00 (0.16) | 1.59 (0.47) | 1.44 (0.36) | 3.26 (1.24) |
| 1000 (484) | 3.25 (0.32) | 1.10 (0.11) | 1.57 (0.45) | 1.90 (0.55) | 3.64 (1.10) |

The GMP side's residues are one `mpz_fdiv_ui` per prime. That is a simpler method than Sagebrush's all-primes-per-word pass, so the residues column compares methods as well as arithmetic.

## Part C: the Python-to-engine boundary

Each big integer crosses as a decimal string in JSON: V8 prints it, dashu parses it, and the reply goes back the same way. Files: `results/vm4/C-*.txt`.

| words | decimal round trip (V8 to and from, dashu to and from) | hex round trip |
|---:|---:|---:|
| 16 | about 5.7 µs | about 1.6 µs |
| 1,024 | about 2.5 ms | about 0.11 ms |
| 5,191 (about $10^5$ digits) | about 28 ms | about 0.73 ms |

## What this says about the plan's order (native first)

1. **Batched multimodular kernels and small-integer overhead (W3, W2) come first.** F4's profile is dominated by them: residues by one word, CRT lifting, building integers from buffers, and allocation.
2. **gcd at small and medium sizes, and rational reconstruction (W6).** gcd is 6 to 12% of F4, and rational reconstruction is the weakest kernel against GMP.
3. **Move ECM's Montgomery arithmetic into the shared crate** as a reusable modular context (W4b), then measure it against GMP's modular powers (dashu's own powmod is 1.7 to 3.2 times GMP with assembly).
4. **Hex at the boundary (W7a)** is cheap and low risk.
5. **The large-size items (W1, W4a, W5) can wait.** No profiled workload spent time at $10^4$ digits or more.

## Still to measure

- modular symbols' profile, with perf started at launch;
- the imaginary quadratic class group's full profile;
- the size histogram of operands in F4 and the class group;
- the mpmath core at high precision, in its own harness;
- the same kernels in wasm, against gmpy2 in Pyodide.
