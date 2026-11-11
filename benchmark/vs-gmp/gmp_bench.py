# The same five operations with GMP (gmpy2): best per-op time over 3 batches of >= 0.2 s.
import random, time, gmpy2
from gmpy2 import mpz
def bench(f):
    t = time.perf_counter(); f(); one = max(time.perf_counter() - t, 1e-7)
    reps = min(max(int(0.2 / one) + 1, 1), 1000000)
    best = 1e9
    for _ in range(3):
        t = time.perf_counter()
        for _ in range(reps): f()
        best = min(best, (time.perf_counter() - t) / reps)
    return best
R = random.Random(1)
print("gmpy2", gmpy2.version(), gmpy2.mp_version())
for d in (1000, 10000, 100000, 1000000):
    bits = -(-d * 3321928 // 1000000 // 64) * 64
    a = mpz(R.getrandbits(bits) | (1 << (bits - 1))); b = mpz(R.getrandbits(bits) | (1 << (bits - 1)))
    n2 = a * b + 12345; s = str(a)
    for name, f in (("mul", lambda: a * b), ("div", lambda: n2 // b), ("sqrt", lambda: gmpy2.isqrt(n2)),
                    ("to_str", lambda: str(a)), ("from_str", lambda: mpz(s))):
        print(d, name, "%.3e" % bench(f), flush=True)
