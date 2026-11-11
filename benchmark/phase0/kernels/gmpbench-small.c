/* Phase 0, part A (native): GMP at 1 to 128 words, the same operations and method as src/main.rs.
   gcc -O2 -o gmpbench-small gmpbench-small.c -lgmp && ./gmpbench-small */
#include <gmp.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

static double now(void) { struct timespec t; clock_gettime(CLOCK_MONOTONIC, &t); return t.tv_sec + 1e-9 * t.tv_nsec; }
static mpz_t a, b, n, r, acc, g, s, t, m, x, e;
static unsigned long w;

static void op(int k) {
  switch (k) {
    case 0: mpz_mul(r, a, b); break;
    case 1: mpz_addmul(acc, a, b); break;
    case 2: mpz_tdiv_q_ui(r, a, w); break;
    case 3: mpz_tdiv_q(r, n, b); break;
    case 4: mpz_gcd(r, a, b); break;
    case 5: mpz_gcdext(g, s, t, a, b); break;
    default: mpz_powm(r, x, e, m); break;
  }
}

int main(void) {
  const char *names[] = {"mul", "addmul", "div1", "div", "gcd", "xgcd", "powmod"};
  const int sizes[] = {1, 2, 4, 8, 16, 32, 64, 128};
  gmp_randstate_t st; gmp_randinit_default(st);
  mpz_inits(a, b, n, r, acc, g, s, t, m, x, e, NULL);
  for (int si = 0; si < 8; si++) {
    int bits = 64 * sizes[si];
    mpz_urandomb(a, st, bits); mpz_setbit(a, bits - 1);
    mpz_urandomb(b, st, bits); mpz_setbit(b, bits - 1);
    mpz_mul(n, a, b); mpz_add_ui(n, n, 12345);
    w = (gmp_urandomb_ui(st, 8) << 56) | 12345;
    mpz_urandomb(m, st, bits); mpz_setbit(m, bits - 1); mpz_setbit(m, 0);
    mpz_urandomb(x, st, bits); mpz_mod(x, x, m);
    mpz_urandomb(e, st, 64); mpz_setbit(e, 63);
    mpz_set_ui(acc, 0);
    for (int k = 0; k < 7; k++) {
      double t0 = now(); op(k); double one = now() - t0; if (one < 1e-8) one = 1e-8;
      long reps = (long)(0.1 / one) + 1; if (reps > 50000000) reps = 50000000;
      double best = 1e30;
      for (int i = 0; i < 3; i++) { t0 = now(); for (long j = 0; j < reps; j++) op(k); double y = (now() - t0) / reps; if (y < best) best = y; }
      printf("%d %s %.4e\n", sizes[si], names[k], best * 1e9);
    }
  }
  return 0;
}
