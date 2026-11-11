# Phase 0: what Sagebrush's integer arithmetic costs, and where

Phase 0 of Sagebrush's big-integer plan (`design/bigint-plan.md` in the Sagebrush repository, item W0). It is measurement only: nothing here changes dashu or Sagebrush. Its report, `REPORT.md`, sets the priority and the order of the rest of the plan.

The native tier comes first.

## What was measured, and what is still planned

**A. Kernels at the common sizes** (`kernels/`). Measured:
- **Sizes:** 1, 2, 4, 8, 16, 32, 64 and 128 words. Larger sizes, $10^3$ to $10^6$ digits, are in `../vs-gmp`.
- **Operations:**
  - multiplication;
  - multiply-and-accumulate ($a \mathrel{+}= bc$);
  - division by one word, and $2n$-by-$n$ division;
  - gcd and extended gcd;
  - modular exponentiation with a fixed modulus.
- **Implementations:** dashu-int, and GMP 6.3.0 with assembly and as generic C.

Planned, not done: FLINT's `fmpz` as a third column.

**B. Batched kernels of multimodular algorithms** (`batched/`). Measured: Sagebrush's code at a pinned commit (`MultiCrt` in `engine/arith/src/crt.rs`, `zmat::ratrecon`), on dashu, against plain GMP loops:
- residues of 1,000 integers modulo 10, 100 and 1,000 primes just below $2^{62}$;
- CRT reconstruction of the same;
- a dot product of length 100 over $\mathbb{Z}$;
- rational reconstruction.

Planned, not done: FLINT's multimodular routines.

**C. The Python-to-engine boundary** (`boundary/`). Measured: four conversions, each timed in isolation, at 1 word to about $10^5$ digits:
- V8 BigInt to and from decimal and hex strings, in node;
- dashu-int to and from decimal and hex, natively.

Not measured: a whole engine call, JSON serialization of a request, and raw words as an alternative to strings.

**D. Real workloads, natively** (`profile/`). Measured: `perf` profiles of native builds, one thread each, sorted into kinds of work by `profile/classify.py`:
- F4 over $\mathbb{Q}$: katsura-8 and cyclic-7;
- a class group in a quintic number field;
- ECM.

Attempted, but not captured or not valid (see `REPORT.md`): modular symbols at a large level, an imaginary quadratic class group, and mpmath at high precision.

Planned, not done:
- the operand-size histogram (from a local, uncommitted build of the facade);
- the profiles in WebAssembly.

## Method

- **Pinning.** Each run records the commits, the toolchains and the CPU. `profile/vm4-run.sh` is the clean run's script, with its log in `results/vm4/run.log`.
- **Timing.** Kernels time preconstructed inputs: the best of 3 batches, each sized from one timed run. Batches target 0.1 s in part A and 0.2 s in parts B and C.
- **Inputs.** The two sides of each comparison draw different random inputs of the same sizes.
- **Clean runs.** The figures in the report are from a quiet VM pinned to one core (`results/vm4/`). The first runs, on a shared machine, are in `results/`.
- **Correctness checks**, on each side's own results:
  - part A (dashu): $n = qb + r$, and the Bezout identity for the extended gcd;
  - part B (Sagebrush): CRT round trips for every input, residues with and without precomputed powers agree, and the rational reconstruction recovers its fraction;
  - part B (GMP): a CRT round trip;
  - part C: decimal and hex round trips on both sides.

  Results are not compared across the two implementations.

## Output

`REPORT.md` and `results/`.
