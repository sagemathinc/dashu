/* The same five operations as bigbench (sagebrush-bigint), straight on GMP's mpz: best per-op time over
   3 batches of >= 0.2 s.  Linked against the assembly or the generic-C libgmp.a. */
#include <gmp.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
static double now(void) { struct timespec t; clock_gettime(CLOCK_MONOTONIC, &t); return t.tv_sec + 1e-9 * t.tv_nsec; }
static mpz_t a, b, n2, r; static char *s;
static void op(int k) {
  switch (k) {
    case 0: mpz_mul(r, a, b); break;
    case 1: mpz_tdiv_q(r, n2, b); break;
    case 2: mpz_sqrt(r, n2); break;
    case 3: { char *t = mpz_get_str(NULL, 10, a); free(t); } break;
    default: mpz_set_str(r, s, 10); break;
  }
}
int main(void) {
  const char *names[] = {"mul", "div", "sqrt", "to_str", "from_str"};
  gmp_randstate_t st; gmp_randinit_default(st); mpz_inits(a, b, n2, r, NULL);
  printf("GMP %s, %d-bit limbs\n", gmp_version, mp_bits_per_limb);
  for (long d = 1000; d <= 1000000; d *= 10) {
    long bits = (long)((d * 3.321928094887362 + 63) / 64) * 64;
    mpz_urandomb(a, st, bits); mpz_setbit(a, bits - 1); mpz_urandomb(b, st, bits); mpz_setbit(b, bits - 1);
    mpz_mul(n2, a, b); mpz_add_ui(n2, n2, 12345); s = mpz_get_str(NULL, 10, a);
    for (int k = 0; k < 5; k++) {
      double t = now(); op(k); double one = now() - t; if (one < 1e-7) one = 1e-7;
      long reps = (long)(0.2 / one) + 1; if (reps > 1000000) reps = 1000000;
      double best = 1e9;
      for (int i = 0; i < 3; i++) { t = now(); for (long j = 0; j < reps; j++) op(k); double x = (now() - t) / reps; if (x < best) best = x; }
      printf("%ld %s %.3e\n", d, names[k], best); fflush(stdout);
    }
    free(s);
  }
  return 0;
}
