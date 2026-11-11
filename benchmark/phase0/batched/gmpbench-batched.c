/* Phase 0, part B (native): the same batched kernels as src/main.rs, written as plain GMP loops with the
   same algorithms (so the comparison is the integer arithmetic, not a different method):
   - residues: mpz_fdiv_ui by each prime;
   - crt: x = sum_j y_j Q_j mod P, y_j = r_j w_j mod p_j, Q_j = P / p_j, w_j = Q_j^-1 mod p_j, then symmetric;
   - dot: mpz_addmul over 100 terms;
   - ratrecon: the half extended Euclidean algorithm, stopped at the first remainder <= nb.
   gcc -O2 -o gmpbench-batched gmpbench-batched.c -lgmp && ./gmpbench-batched */
#include <gmp.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

static double now(void) { struct timespec t; clock_gettime(CLOCK_MONOTONIC, &t); return t.tv_sec + 1e-9 * t.tv_nsec; }

static int is_prime(uint64_t n) { mpz_t z; mpz_init_set_ui(z, n); int r = mpz_probab_prime_p(z, 30) > 0; mpz_clear(z); return r; }

#define N 1000
static int K, W;
static uint64_t P[1000], WJ[1000];
static mpz_t X[N], PROD, HALF, Q[1000], A[100], B[100], U, BOUND, T, S;
static uint64_t R[N][1000];
static volatile uint64_t sink;

static void residues(void) { for (int i = 0; i < N; i++) for (int j = 0; j < K; j++) R[i][j] = mpz_fdiv_ui(X[i], P[j]); }
static void crt(void) {
  for (int i = 0; i < N; i++) {
    mpz_set_ui(T, 0);
    for (int j = 0; j < K; j++) {
      uint64_t y = (uint64_t)(((unsigned __int128)R[i][j] * WJ[j]) % P[j]);
      mpz_addmul_ui(T, Q[j], y);
    }
    mpz_mod(T, T, PROD);
    if (mpz_cmp(T, HALF) > 0) mpz_sub(T, T, PROD);
    sink += mpz_sizeinbase(T, 2);
  }
}
static void dot(void) { mpz_set_ui(S, 0); for (int i = 0; i < 100; i++) mpz_addmul(S, A[i], B[i]); sink += mpz_sizeinbase(S, 2); }
static void ratrecon(void) {
  mpz_t r0, r1, t0, t1, q, tmp;
  mpz_inits(r0, r1, t0, t1, q, tmp, NULL);
  mpz_set(r0, PROD); mpz_mod(r1, U, PROD); mpz_set_ui(t0, 0); mpz_set_ui(t1, 1);
  while (mpz_cmp(r1, BOUND) > 0) {
    mpz_fdiv_qr(q, tmp, r0, r1); mpz_swap(r0, r1); mpz_swap(r1, tmp);
    mpz_mul(tmp, q, t1); mpz_sub(tmp, t0, tmp); mpz_swap(t0, t1); mpz_swap(t1, tmp);
  }
  sink += mpz_sizeinbase(t1, 2);
  mpz_clears(r0, r1, t0, t1, q, tmp, NULL);
}

static double timeit(void (*f)(void), double per) {
  double t0 = now(); f(); double one = now() - t0; if (one < 1e-7) one = 1e-7;
  long reps = (long)(0.2 / one) + 1; if (reps > 1000000) reps = 1000000;
  double best = 1e30;
  for (int i = 0; i < 3; i++) { t0 = now(); for (long j = 0; j < reps; j++) f(); double y = (now() - t0) / reps; if (y < best) best = y; }
  return best * 1e9 / per;
}

int main(void) {
  gmp_randstate_t st; gmp_randinit_default(st);
  for (int i = 0; i < N; i++) mpz_init(X[i]);
  for (int i = 0; i < 1000; i++) mpz_init(Q[i]);
  for (int i = 0; i < 100; i++) { mpz_init(A[i]); mpz_init(B[i]); }
  mpz_inits(PROD, HALF, U, BOUND, T, S, NULL);
  int ks[] = {10, 100, 1000};
  for (int ki = 0; ki < 3; ki++) {
    K = ks[ki]; W = (K * 62) / 64 / 2; if (W < 1) W = 1;
    uint64_t p = (uint64_t)1 << 62;
    for (int j = 0; j < K; j++) { do p--; while (!is_prime(p)); P[j] = p; }
    mpz_set_ui(PROD, 1); for (int j = 0; j < K; j++) mpz_mul_ui(PROD, PROD, P[j]);
    mpz_fdiv_q_2exp(HALF, PROD, 1);
    for (int j = 0; j < K; j++) {
      mpz_divexact_ui(Q[j], PROD, P[j]);
      mpz_t m, pj; mpz_init_set_ui(pj, P[j]); mpz_init(m);
      mpz_mod_ui(m, Q[j], P[j]); mpz_invert(m, m, pj); WJ[j] = mpz_get_ui(m);
      mpz_clears(m, pj, NULL);
    }
    for (int i = 0; i < N; i++) { mpz_urandomb(X[i], st, 64 * W - 1); mpz_setbit(X[i], 64 * W - 2); if (i & 1) mpz_neg(X[i], X[i]); }
    for (int i = 0; i < 100; i++) { mpz_urandomb(A[i], st, 64 * W - 1); mpz_urandomb(B[i], st, 64 * W - 1); }
    int hw = (K * 31 - 2) / 64; if (hw < 1) hw = 1;
    mpz_set_ui(BOUND, 1); mpz_mul_2exp(BOUND, BOUND, 64 * hw);
    { mpz_t num, den; mpz_inits(num, den, NULL);
      mpz_urandomb(num, st, 64 * hw - 1); mpz_urandomb(den, st, 64 * hw - 1); mpz_setbit(den, 0);
      while (!mpz_invert(U, den, PROD)) mpz_add_ui(den, den, 2);
      mpz_mul(U, U, num); mpz_mod(U, U, PROD);
      mpz_clears(num, den, NULL); }
    /* correctness: the CRT round trip */
    residues(); crt();
    { mpz_set_ui(T, 0); for (int j = 0; j < K; j++) { uint64_t y = (uint64_t)(((unsigned __int128)R[0][j] * WJ[j]) % P[j]); mpz_addmul_ui(T, Q[j], y); }
      mpz_mod(T, T, PROD); if (mpz_cmp(T, HALF) > 0) mpz_sub(T, T, PROD);
      if (mpz_cmp(T, X[0])) { fprintf(stderr, "CRT round trip failed\n"); return 1; } }
    printf("residues %d %d %.4e\n", K, W, timeit(residues, N));
    printf("crt %d %d %.4e\n", K, W, timeit(crt, N));
    printf("dot %d %d %.4e\n", K, W, timeit(dot, 100));
    printf("ratrecon %d %d %.4e\n", K, W, timeit(ratrecon, 1));
  }
  return 0;
}
