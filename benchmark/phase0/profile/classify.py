"""Phase 0, part D: sort a workload's perf profile (perf report --stdio --no-children --sort dso,sym, self time)
into kinds of work. Usage: python3 classify.py results/D-*.report.txt"""
import re, sys

RULES = [  # (kind, test on "dso symbol"); first match wins
    ("bigint: gcd", lambda s: re.search(r"(dashu|sagebrush_bigint).*(gcd|Gcd|hgcd)", s)),
    ("bigint: division and remainder", lambda s: re.search(r"dashu_int::div|rem_by|div_by|DivRem|Rem>|Div>|__udivti3|__umodti3", s)),
    ("bigint: multiplication", lambda s: re.search(r"dashu_int::mul|mul_ibig|Mul>::mul|mul_large|sagebrush_bigint::ntt", s)),
    ("bigint: other (add, conversion, representation)", lambda s: re.search(r"dashu_int::|dashu_base::|sagebrush_bigint::dashu_impl", s)),
    ("word-size modular arithmetic", lambda s: re.search(r"sagebrush_bigint::nmod|sagebrush_arith::(spelim|nmod)|Modulus|dense_axpy|mulmod|ntt", s)),
    ("allocation and copying", lambda s: re.search(r"\b(malloc|_int_malloc|_int_free|cfree|free|realloc|_int_realloc|calloc|__memmove\w*|__memcpy\w*|__memset\w*|memcpy|memmove|memset)\b", s)),
    ("Python interpreter", lambda s: re.search(r"python3|libpython", s)),
]


def classify(path):
    kinds = {}
    for line in open(path):
        m = re.match(r"\s*([\d.]+)%\s+(\S+)\s+\[\.\]\s+(.*)", line)
        if not m:
            continue
        pct, dso, sym = float(m.group(1)), m.group(2), m.group(3)
        s = dso + " " + sym
        kind = next((k for k, t in RULES if t(s)), "the algorithm's own code (" + dso + ")" if "_native" in dso else "other (" + dso + ")")
        kinds[kind] = kinds.get(kind, 0.0) + pct
    return kinds


for p in sys.argv[1:]:
    k = classify(p)
    name = p.split("D-")[-1].replace(".report.txt", "")
    big = sum(v for kk, v in k.items() if kk.startswith("bigint"))
    print("## %s: bigint %.1f%%" % (name, big))
    for kk, v in sorted(k.items(), key=lambda x: -x[1]):
        if v >= 0.5:
            print("  %5.1f%%  %s" % (v, kk))
