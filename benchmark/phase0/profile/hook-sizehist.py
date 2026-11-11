"""Phase 0 only: insert the size-histogram hooks into engine/bigint (local, uncommitted)."""
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
H = "crate::sizehist::rec"
W = "crate::sizehist::words"
R = []
def r(a, b, n=1):
    R.append((a, b, n))
# mul_ibig: every BigInt product
r("pub fn mul_ibig(a: &IBig, b: &IBig) -> IBig {\n",
  "pub fn mul_ibig(a: &IBig, b: &IBig) -> IBig {\n    %s(\"Mul\", %s(a), %s(b));\n" % (H, W, W))
# binop macro (Add, Sub, Div, Rem, bit ops): record before the operation
for lhs, rhs in [("self.0", "r.0"), ("self.0", "&r.0"), ("&self.0", "r.0"), ("&self.0", "&r.0")]:
    r("                BigInt(%s $op %s)\n" % (lhs, rhs),
      "                %s(stringify!($tr), %s(&self.0), %s(&r.0));\n                BigInt(%s $op %s)\n" % (H, W, W, lhs, rhs))
r("                let a = std::mem::take(&mut self.0);\n                self.0 = a $op r.0;\n",
  "                %s(stringify!($tr), %s(&self.0), %s(&r.0));\n                let a = std::mem::take(&mut self.0);\n                self.0 = a $op r.0;\n" % (H, W, W))
r("                let a = std::mem::take(&mut self.0);\n                self.0 = a $op &r.0;\n",
  "                %s(stringify!($tr), %s(&self.0), %s(&r.0));\n                let a = std::mem::take(&mut self.0);\n                self.0 = a $op &r.0;\n" % (H, W, W))
# binop_prims: one big operand, one primitive
for lhs in ["self.0", "&self.0"]:
    r("                BigInt(%s $op IBig::from(r))\n" % lhs,
      "                %s(concat!(stringify!($tr), \"_prim\"), %s(&self.0), 1);\n                BigInt(%s $op IBig::from(r))\n" % (H, W, lhs))
for rhs in ["r.0", "&r.0"]:
    r("                BigInt(IBig::from(self) $op %s)\n" % rhs,
      "                %s(concat!(stringify!($tr), \"_prim\"), 1, %s(&r.0));\n                BigInt(IBig::from(self) $op %s)\n" % (H, W, rhs))
r("                let a = std::mem::take(&mut self.0);\n                self.0 = a $op IBig::from(r);\n",
  "                %s(concat!(stringify!($tr), \"_prim\"), %s(&self.0), 1);\n                let a = std::mem::take(&mut self.0);\n                self.0 = a $op IBig::from(r);\n" % (H, W))
# Integer: div_rem, div_mod_floor, gcd
r("    fn div_mod_floor(&self, d: &Self) -> (Self, Self) {\n        let (q, r) = (&self.0).div_rem(&d.0);\n",
  "    fn div_mod_floor(&self, d: &Self) -> (Self, Self) {\n        %s(\"div_rem\", %s(&self.0), %s(&d.0));\n        let (q, r) = (&self.0).div_rem(&d.0);\n" % (H, W, W))
r("    fn div_rem(&self, d: &Self) -> (Self, Self) {\n        let (q, r) = (&self.0).div_rem(&d.0);\n        (BigInt(q), BigInt(r))\n",
  "    fn div_rem(&self, d: &Self) -> (Self, Self) {\n        %s(\"div_rem\", %s(&self.0), %s(&d.0));\n        let (q, r) = (&self.0).div_rem(&d.0);\n        (BigInt(q), BigInt(r))\n" % (H, W, W))
r("    fn gcd(&self, other: &Self) -> Self {\n        // dashu panics for gcd(0, 0); num-bigint's is 0\n",
  "    fn gcd(&self, other: &Self) -> Self {\n        %s(\"gcd\", %s(&self.0), %s(&other.0));\n        // dashu panics for gcd(0, 0); num-bigint's is 0\n" % (H, W, W))
# conversions
r("    pub fn to_u64_digits(&self) -> (Sign, Vec<u64>) {\n",
  "    pub fn to_u64_digits(&self) -> (Sign, Vec<u64>) {\n        %s(\"conv\", %s(&self.0), 0);\n" % (H, W))
r("    pub fn from_bytes_le(sign: Sign, bytes: &[u8]) -> BigInt {\n",
  "    pub fn from_bytes_le(sign: Sign, bytes: &[u8]) -> BigInt {\n        %s(\"conv\", (bytes.len() + 7) / 8, 0);\n" % H)
for a, b, n in R:
    c = s.count(a)
    if c < 1:
        sys.exit("missing: %r" % a[:70])
    s = s.replace(a, b)
open(p, "w").write(s)
print("ok", len(R), "hooks")
