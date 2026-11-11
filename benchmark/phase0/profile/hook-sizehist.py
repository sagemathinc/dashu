"""Phase 0 only: insert the size-histogram hooks into engine/bigint (local, uncommitted), covering every
arithmetic entry point of the facade: the BigInt and BigUint operators (with references, primitives and
assignment forms), products, shifts, and the named methods. Conversions are counted under "conv",
which the summary keeps apart from arithmetic. Comparisons, negation and absolute value are not counted.
Usage: python3 hook-sizehist2.py SAGEBRUSH_ROOT (on a clean engine/bigint)"""
import re, sys
root = sys.argv[1]
lib = root + "/engine/bigint/src/lib.rs"
s = open(lib).read()
a = "#[cfg(not(feature = \"num-backend\"))]\nmod dashu_impl;"
assert s.count(a) == 1
s = s.replace(a, a + "\n#[cfg(not(feature = \"num-backend\"))]\npub mod sizehist;")
open(lib, "w").write(s)

p = root + "/engine/bigint/src/dashu_impl.rs"
s = open(p).read()
H, W = "crate::sizehist::rec", "crate::sizehist::words"
n_total = 0

def sub_all(pattern, repl, min_count=1):
    global s, n_total
    s, n = re.subn(pattern, repl, s, flags=re.M)
    if n < min_count:
        sys.exit("pattern matched %d < %d: %s" % (n, min_count, pattern))
    n_total += n

# 1. binary operators on two big operands, both types: `T(L $op R)` (L, R in self.0 / &self.0 / r.0 / &r.0)
sub_all(r"^(\s+)(BigInt|BigUint)\((&?self\.0) \$op (&?r\.0)\)$",
        r'\1%s(stringify!($tr), %s(&self.0), %s(&r.0));\n\1\2(\3 $op \4)' % (H, W, W), 8)
# 2. assignment forms with a big right operand: `let a = take(...); self.0 = a $op r.0;`
sub_all(r"^(\s+)let a = std::mem::take\(&mut self\.0\);\n(\s+)self\.0 = a \$op (&?r\.0);$",
        r'\1%s(stringify!($tr), %s(&self.0), %s(&r.0));\n\1let a = std::mem::take(&mut self.0);\n\2self.0 = a $op \3;' % (H, W, W), 4)
# 3. primitive right operand: `T(L $op IBig::from(r))`, `T(L $op UBig::from(r))`, and the assignment form
sub_all(r"^(\s+)(BigInt|BigUint)\((&?self\.0) \$op (IBig|UBig)::from\(r\)\)$",
        r'\1%s(concat!(stringify!($tr), "_prim"), %s(&self.0), 1);\n\1\2(\3 $op \4::from(r))' % (H, W), 4)
sub_all(r"^(\s+)let a = std::mem::take\(&mut self\.0\);\n(\s+)self\.0 = a \$op (IBig|UBig)::from\(r\);$",
        r'\1%s(concat!(stringify!($tr), "_prim"), %s(&self.0), 1);\n\1let a = std::mem::take(&mut self.0);\n\2self.0 = a $op \3::from(r);' % (H, W), 2)
# 4. primitive left operand: `T(IBig::from(self) $op r.0)`
sub_all(r"^(\s+)(BigInt|BigUint)\((IBig|UBig)::from\(self\) \$op (&?r\.0)\)$",
        r'\1%s(concat!(stringify!($tr), "_prim"), 1, %s(&r.0));\n\1\2(\3::from(self) $op \4)' % (H, W), 2)
# 5. products: mul_ibig / mul_ubig called from the Mul impls (record at the call sites, once per product)
sub_all(r"^(\s+)(BigInt|BigUint)\((mul_[iu]big)\(&self\.0, &r\.0\)\)$",
        r'\1%s("Mul", %s(&self.0), %s(&r.0));\n\1\2(\3(&self.0, &r.0))' % (H, W, W), 8)
sub_all(r"^(\s+)self\.0 = (mul_[iu]big)\(&self\.0, &r\.0\);$",
        r'\1%s("Mul", %s(&self.0), %s(&r.0));\n\1self.0 = \2(&self.0, &r.0);' % (H, W, W), 4)
# 6. shifts: `$t(self.0 << ...)` / `$t(&self.0 >> ...)`
sub_all(r"^(\s+)\$t\((&?self\.0) (<<|>>) (.*)\)$",
        lambda m: '%s%s("%s", %s(&self.0), 0);\n%s$t(%s %s %s)' % (m.group(1), H, "Shl" if m.group(3) == "<<" else "Shr", W, m.group(1), m.group(2), m.group(3), m.group(4)), 4)
# 7. named methods: record at entry (self, and the other big operand where there is one)
methods = {
    "pow": None, "sqrt": None, "nth_root": None, "cbrt": None,
    "gcd": "other", "lcm": "other", "extended_gcd": "other", "div_rem": "d", "div_mod_floor": "d",
    "modpow": "m", "modinv": "m", "div_floor": "d", "mod_floor": "d", "is_multiple_of": "other",
}
def method_hook(m):
    indent, sig, name = m.group(1), m.group(2), m.group(3)
    other = methods[name]
    o = "%s(&%s.0)" % (W, other) if other else "0"
    return '%s%s {\n%s    %s("%s", %s(&self.0), %s);' % (indent, sig, indent, H, name, W, o)
sub_all(r"^(\s+)((?:pub )?fn (%s)\(&?self[^)]*\)[^{]*)\{$" % "|".join(methods), method_hook, 10)
# BigInt's div_floor and mod_floor go through its counted div_mod_floor: no second hook there
sub_all(r"^\s+crate::sizehist::rec\(\"(div_floor|mod_floor)\", [^\n]*\n(\s+self\.div_mod_floor\(d\))", r"\2", 2)
# 8. conversions (kept apart from arithmetic by the summary)
sub_all(r"^(\s+)pub fn to_u64_digits\(&self\) -> \(Sign, Vec<u64>\) \{$",
        r'\1pub fn to_u64_digits(&self) -> (Sign, Vec<u64>) {\n\1    %s("conv", %s(&self.0), 0);' % (H, W), 1)
sub_all(r"^(\s+)pub fn from_bytes_le\(sign: Sign, bytes: &\[u8\]\) -> BigInt \{$",
        r'\1pub fn from_bytes_le(sign: Sign, bytes: &[u8]) -> BigInt {\n\1    %s("conv", (bytes.len() + 7) / 8, 0);' % H, 1)
open(p, "w").write(s)
print("ok", n_total, "hooks")
