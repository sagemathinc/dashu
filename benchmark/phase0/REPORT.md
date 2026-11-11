# Phase 0 report: what Sagebrush's integer arithmetic costs, and where

Phase 0 of Sagebrush's big-integer plan (`design/bigint-plan.md`, item W0). It is measurement only. The method is in `README.md`, the raw outputs in `results/` (first runs on a shared machine) and `results/vm4/` (clean runs).

**Clean runs.** All figures below come from one quiet VM: an AMD EPYC 7B13 pinned to one core (core 3), Nov 11 2026, 07:35-07:44Z. Sagebrush was at master `4d4225f9`; the integer code is dashu-int 0.6.2 under `sagebrush-bigint`. The first runs on the shared project machine agree with these within noise.

## The answer, in short

- **Integer arithmetic is a large share of real exact workloads' native time.**
  - F4 over $\mathbb{Q}$ spends **55 to 61%** of its single-thread time in big-integer arithmetic, plus 13 to 16% in allocation and copying.
  - A class group computation in a quintic number field spends **66%** there.
  - By the plan's stop rule, that is a reason to go on past Phase 2.
- **The hot functions are the multimodular steps' small-operand ones.** In F4 the top items are:
  - gcds, for normalizing rationals;
  - the CRT lift;
  - building integers from limb buffers;
  - remainders by one word (the reductions mod each prime);
  - `malloc`/`free`;
  - one-word-by-many multiplications.
- **The operands are small: 1 to 63 words (up to about 1,200 digits) almost everywhere.** A size histogram of the facade's operations (below) puts F4's operands at 15 words or less (katsura-8) and 31 or less (cyclic-7), mostly 2 to 7 words. The quintic class group is the only workload above 63 words: 0.5% of its operations, but about 10% of its operand words, some of them at 5,000 to 10,000 digits.
- **Two workloads spend little time on big integers.**
  - Modular symbols (level 10007): 84% of its time is in its own word-size modular linear algebra (`sagebrush_modsym::linalg`: `dot_lazy`, `sub_mul`, `acc_mul`, all AVX2), and only about 0.2% in big-integer arithmetic.
  - The imaginary quadratic class group: about 10% in big integers, with most of its time in its own relation search and linear algebra.
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
- **At the Python-to-engine boundary, decimal conversion costs 3.3 to 37 times what hex would**, from 16 to 5,191 words. That is the conversion component alone, timed in isolation, not a measured whole-boundary saving: big integers cross as decimal strings in JSON, and the engine call's other costs were not measured.

## Part D: where real workloads spend their time

Native, one thread (`SAGEBRUSH_THREADS=1`), pinned to one core, profiled with `perf` at 997 Hz (self time). Each workload ran through the native Python package built from master `4d4225f9` with line tables. `profile/classify.py` sorts the samples into kinds of work.

| workload | wall | integer arithmetic | allocation and copying | word-size modular arithmetic | the algorithm's own code |
|---|---:|---:|---:|---:|---:|
| F4 over $\mathbb{Q}$, katsura-8 | 12.2 s | **60.6%** | 16.0% | 4.8% | 16.0% |
| F4 over $\mathbb{Q}$, cyclic-7 | 8.4 s | **55.0%** | 13.3% | 12.3% | 16.7% |
| class group, $x^5 - 3x^4 + 7x - 1234567$ | 6.9 s | **65.9%** | 8.1% | 0 | 24.4% |
| ECM, three primes near $10^{23}$, $10^{25}$, $10^{27}$ | 14.9 s | 0 in dashu; **99%** in ECM's own Montgomery arithmetic | 0 | 0 | 99.8% |
| modular symbols, level 10007, $T_2$'s characteristic polynomial | 16.4 s | about 0.2% | 2.0% | **84%** in its own modular linear algebra, plus 2.2% `__umodti3` (a 128-bit remainder at the word level) | 90.2% (the linear algebra included) |
| class group, $\mathbb{Q}(\sqrt{-(10^{36}+237)})$ | 1.5 s | 9.7% | 6.8% | 0 | 64.1% |

The last two were profiled in a second run, sampling the pinned core for the workload's whole lifetime (`perf record -a -C 3`), because their work runs on threads that start after launch. The first run's attach-by-process (`perf record -p`) missed those threads. Files: `results/vm4-run2/`. The classifier counts `__umodti3` and `__udivti3` (compiler helpers for 128-bit division) as integer division; in modular symbols they come from word-size arithmetic, so the table above moves them to that column.

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

**Not captured:**
- **mpmath at high precision.** The native package ran CPython's own mpmath on Python integers (the VM has no gmpy2), not Sagebrush's multiprecision core, so the 64 s run says nothing about our integers. The core needs its own harness.

### Operand sizes

A local measuring patch to Sagebrush's integer facade at `4d4225f9` (`profile/size-hist.patch`, never part of Sagebrush) counts every arithmetic operation on its `BigInt` and `BigUint` by the size of the larger operand, with buckets of $[2^{b-1}, 2^b)$ words. The workloads are the same, run natively on one thread. These are counts, not times, so the "operand words" column (the sum of both operands' words) is only a rough proxy for cost. Files: `results/hist/`; summary: `profile/hist-summary.py`.

| workload | operations | where the operations are (by larger operand) | the largest bucket reached |
|---|---:|---|---|
| F4 over $\mathbb{Q}$, katsura-8 | 173 million | 4-7 words: 69% of operations, 81% of operand words; 2-3 words: 27% | 8-15 words (0.4%) |
| F4 over $\mathbb{Q}$, cyclic-7 | 136 million | 4-7 words: 42%; 2-3 words: 27%; 8-15 words: 13%; 1 word: 14% | 16-31 words (5%) |
| class group, quintic | 79 million | 1 word: 78% of operations; by operand words, 32-63 words: 57% | 256-511 words (0.1% of operations, 3.9% of operand words); 64-127 words: 0.4% of operations, 6.0% of operand words |
| class group, imaginary quadratic | 1.6 million | 2-3 words: 78%; 4-7 words: 19% | 16-31 words (under 0.05%) |
| modular symbols, level 10007 | 0.7 million (all additions) | 16-31 words: 42%; 8-15 words: 35% | 16-31 words |
| ECM | 27 thousand | 1-7 words | 8-15 words (0.5%) |

The histogram sees only the facade. ECM's own Montgomery arithmetic, the word-level work inside `MultiCrt`, and any direct use of dashu in an engine crate are not counted.

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

At 2 to 4 words, multiplication is 2.7 to 3.3 times GMP with assembly, about the same as generic GMP. That suggests per-operation overhead rather than the multiply loop, but these runs do not isolate allocation, dispatch, code generation or thresholds. From 8 words up the gap to GMP with assembly is about 1.5 to 2 times, consistent with GMP's assembly loops.

The timing scopes differ: dashu allocates each result, while GMP writes into a reused variable, and GMP's `addmul` is one fused call where dashu does a multiply and an add. The two sides also draw different random inputs of the same sizes. So the ratios compare the two implementations as used, not their inner loops alone.

## Part B: Sagebrush's multimodular kernels

Sagebrush's own code (`MultiCrt`, `zmat::ratrecon` at `4d4225f9`) against plain GMP loops: the same method for CRT reconstruction, a different one for residues (below), a multiply-plus-add against GMP's fused `addmul` for the dot product, and the same half extended Euclidean algorithm on separately generated fractions for rational reconstruction. The inputs are 1,000 integers filling half the product of $k$ primes just below $2^{62}$. Ratios are Sagebrush ÷ GMP with assembly; in parentheses, ÷ generic C. Files: `results/vm4/B-*.txt`.

| $k$ (words) | residues | residues, precomputed powers | CRT | dot product | rational reconstruction |
|---|---|---|---|---|---|
| 10 (4) | 1.12 (0.61) | 0.90 (0.49) | 2.57 (1.18) | 3.15 (1.25) | 2.41 (1.62) |
| 100 (48) | 1.98 (0.31) | 1.00 (0.16) | 1.59 (0.47) | 1.44 (0.36) | 3.26 (1.24) |
| 1000 (484) | 3.25 (0.32) | 1.10 (0.11) | 1.57 (0.45) | 1.90 (0.55) | 3.64 (1.10) |

The GMP side's residues are one `mpz_fdiv_ui` per prime. That is a simpler method than Sagebrush's all-primes-per-word pass, so the residues column compares methods as well as arithmetic.

## Part C: the Python-to-engine boundary

Each big integer crosses as a decimal string in JSON: V8 prints it, dashu parses it, and the reply goes back the same way. The table sums four conversions timed in isolation (V8 to and from a string, dashu to and from a string); it does not include the engine call, JSON serialization of the request, or transport. Files: `results/vm4/C-*.txt`.

| words | decimal conversions, summed | hex conversions, summed | ratio |
|---:|---:|---:|---:|
| 16 | 5.9 µs | 1.8 µs | 3.3 |
| 1,024 | 2.5 ms | 0.13 ms | 19.8 |
| 5,191 (about $10^5$ digits) | 28 ms | 0.74 ms | 37.2 |

## What this says about the plan's order (native first)

1. **Batched multimodular kernels and small-integer overhead (W3, W2) come first** (a provisional reading of the hot functions): F4's profile is dominated by residues by one word, CRT lifting, building integers from buffers, and allocation.
2. **gcd at small and medium sizes, and rational reconstruction (W6).** gcd is 6 to 12% of F4, and rational reconstruction is the weakest kernel against GMP.
3. **Move ECM's Montgomery arithmetic into the shared crate** as a reusable modular context (W4b), then measure it against GMP's modular powers (dashu's own powmod is 1.7 to 3.2 times GMP with assembly).
4. **Hex at the boundary (W7a)** is cheap and low risk.
5. **The large-size items (W1, W4a, W5) can wait.** In the profiled exact workloads, operands stay below about 1,200 digits except in the quintic class group (0.5% of its operations, about 10% of its operand words). The mpmath core at high precision is the one place they would matter, and it is still unmeasured.

## Still to measure

- the mpmath core at high precision, in its own harness;
- the same kernels in wasm, against gmpy2 in Pyodide.
