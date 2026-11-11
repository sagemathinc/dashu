# Phase 0: what Sagebrush's integer arithmetic costs, and where

Phase 0 of Sagebrush's big-integer plan (`design/bigint-plan.md` in the Sagebrush repository, item W0). It is measurement only: nothing here changes dashu or Sagebrush. Its report sets the priority and the order of the rest of the plan.

The native tier comes first. The browser numbers are kept for comparison.

## What is measured

**A. Kernels at the common sizes.**
- **Sizes:** 1, 2, 4, 8, 16, 32 and 64 words, then $10^3$ to $10^6$ digits as in `../vs-gmp`.
- **Operations:**
  - multiplication;
  - multiply-and-accumulate ($a \mathrel{+}= bc$);
  - division by one word, and $2n$-by-$n$ division;
  - gcd and extended gcd;
  - modular exponentiation with a fixed modulus.
- **Implementations:**
  - dashu-int;
  - GMP 6.3.0, with assembly and as generic C;
  - FLINT's `fmpz`, as a benchmark binary only, never linked into Sagebrush.

**B. Batched kernels of multimodular algorithms.**
- residues of 1,000 integers of each size modulo 10, 100 and 1,000 word-size primes;
- CRT reconstruction of the same;
- a dot product of length 100 over $\mathbb{Z}$;
- rational reconstruction.

Implementations: Sagebrush's current code path, `engine/arith/src/crt.rs` on dashu, against FLINT's multimodular routines and plain GMP loops.

**C. The Python-to-engine boundary.** The cost of passing integers of 1 word to $10^5$ digits through the engine's JSON requests as decimal, compared with hex and raw words. Measured in Sagebrush's runtime.

**D. Real workloads, natively.** For each workload, the share of time spent in integer arithmetic (dashu, the `sagebrush-bigint` facade, word-size modular arithmetic, and other code), and the sizes of the operands involved:
- F4 over $\mathbb{Q}$: katsura-9 and cyclic-7;
- modular symbols at a large level;
- a class group;
- ECM;
- the mpmath core's performance rows.

Shares come from `perf` profiles of native builds. Operand sizes come from a size histogram in a local, uncommitted build of the facade.

Each workload is also profiled once in WebAssembly, for comparison.

## Method

- **Pinning.** Every run records the commit of each code base, the toolchains, the CPU and the timing scope, as `../vs-gmp` does.
- **Timing.** Kernels time preconstructed inputs, best of 3 batches, each batch targeting 0.2 s.
- **Clean runs.** Timings that matter are rerun on a quiet VM.
- **Correctness.** Every kernel's result is checked against GMP's.

## Output

`REPORT.md` and `results/`.
