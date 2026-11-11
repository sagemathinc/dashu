// Phase 0, part C: the Python runtime's side of the engine boundary. Python ints past 2^53 are V8 BigInts;
// the engine's JSON requests carry them as decimal strings. This times BigInt -> string and string -> BigInt
// in decimal and in hex, at 1 word to 10^5 digits. Prints `words op ns`.
// node v8-bigint.mjs
const sizes = [1, 4, 16, 64, 256, 1024, 5191];  // 5191 words is about 10^5 digits
let seed = 0x9e3779b97f4a7c15n;
function rnd(words) {
  let x = 0n;
  for (let i = 0; i < words; i++) {
    seed ^= (seed << 13n) & 0xffffffffffffffffn; seed ^= seed >> 7n; seed ^= (seed << 17n) & 0xffffffffffffffffn;
    x = (x << 64n) | seed;
  }
  return x | (1n << BigInt(64 * words - 1));
}
function time(f) {
  let t = performance.now(); f(1); const one = Math.max(performance.now() - t, 1e-4);
  const reps = Math.min(Math.max(Math.ceil(200 / one), 1), 5e6);
  let best = Infinity;
  for (let i = 0; i < 3; i++) { t = performance.now(); f(reps); best = Math.min(best, (performance.now() - t) / reps); }
  return best * 1e6;
}
let sink = 0;
for (const w of sizes) {
  const x = rnd(w), dec = x.toString(10), hex = x.toString(16), hex0 = "0x" + hex;
  if (BigInt(dec) !== x || BigInt(hex0) !== x) throw new Error("round trip");
  const rows = {
    to_dec: time((r) => { for (let i = 0; i < r; i++) sink += x.toString(10).length; }),
    from_dec: time((r) => { for (let i = 0; i < r; i++) sink += Number(BigInt(dec) & 1n); }),
    to_hex: time((r) => { for (let i = 0; i < r; i++) sink += x.toString(16).length; }),
    from_hex: time((r) => { for (let i = 0; i < r; i++) sink += Number(BigInt(hex0) & 1n); }),
    json_dec: time((r) => { for (let i = 0; i < r; i++) sink += JSON.stringify({ n: dec }).length; }),
  };
  for (const [op, ns] of Object.entries(rows)) console.log(`${w} ${op} ${ns.toExponential(4)}`);
}
if (sink === 42) console.log("");
