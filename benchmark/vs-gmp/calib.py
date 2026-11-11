# Calibration: Python's own per-call cost, CPython ints vs gmpy2 mpz at the same sizes.
import random, time, sys, gmpy2
from gmpy2 import mpz
def bench(f):
    t = time.perf_counter(); f(); one = max(time.perf_counter() - t, 1e-7)
    reps = min(max(int(0.2 / one) + 1, 1), 2000000); best = 1e9
    for _ in range(3):
        t = time.perf_counter()
        for _ in range(reps): f()
        best = min(best, (time.perf_counter() - t) / reps)
    return best
R = random.Random(1)
print(sys.version.split()[0], sys.platform, "gmpy2", gmpy2.version(), gmpy2.mp_version(), "limb bits", gmpy2.mp_limbsize() if hasattr(gmpy2, "mp_limbsize") else "?")
print("empty lambda %.3e" % bench(lambda: None))
for d in (1000, 10000, 100000):
    bits = d * 3322 // 1000
    x = R.getrandbits(bits) | 1 << (bits - 1); y = R.getrandbits(bits) | 1 << (bits - 1)
    X, Y = mpz(x), mpz(y)
    print(d, "int mul %.3e" % bench(lambda: x * y), " mpz mul %.3e" % bench(lambda: X * Y), " mpz add %.3e" % bench(lambda: X + Y), flush=True)
