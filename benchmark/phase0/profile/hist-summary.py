"""Phase 0, part D: summarize an operand-size histogram (SB_SIZE_HIST output of the instrumented facade).
Bucket b holds operands of [2^(b-1), 2^b) words (the larger operand); words_sum adds both operands' words.
Usage: python3 hist-summary.py results/hist/*.hist"""
import sys
from collections import defaultdict

DIGITS_PER_WORD = 19.27


def label(b):
    if b == 0:
        return "0"
    lo, hi = 2 ** (b - 1), 2 ** b - 1
    return "%d" % lo if lo == hi else "%d-%d" % (lo, hi)


for path in sys.argv[1:]:
    name = path.split("/")[-1].replace(".hist", "")
    count = defaultdict(lambda: defaultdict(int))
    words = defaultdict(lambda: defaultdict(int))
    conv = defaultdict(int)
    for line in open(path):
        f = line.split()
        if len(f) != 5 or f[1] == "Mul2":
            continue
        _, op, b, c, w = f
        if op == "conv":  # conversions to and from words or bytes: reported apart from arithmetic
            conv[int(b)] += int(c)
            continue
        count[op][int(b)] += int(c)
        words[op][int(b)] += int(w)
    total = sum(sum(v.values()) for v in count.values())
    tw = sum(sum(v.values()) for v in words.values())
    print("## %s: %d arithmetic operations, %d operand words in all" % (name, total, tw))
    by_b = defaultdict(int)
    by_bw = defaultdict(int)
    for op in count:
        for b, c in count[op].items():
            by_b[b] += c
            by_bw[b] += words[op][b]
    print("  larger operand (words) | share of operations | share of operand words (a cost proxy)")
    for b in sorted(by_b):
        print("  %-12s (<= %6.0f digits) | %5.1f%% | %5.1f%%" % (label(b), (2 ** b - 1) * DIGITS_PER_WORD if b else 0, 100.0 * by_b[b] / total, 100.0 * by_bw[b] / max(tw, 1)))
    top = sorted(((sum(v.values()), op) for op, v in count.items()), reverse=True)[:8]
    if conv:
        print("  conversions (not counted above): %d, larger bucket %s words" % (sum(conv.values()), label(max(conv))))
    print("  operations: " + ", ".join("%s %.1f%%" % (op, 100.0 * c / total) for c, op in top))
