// node chrome-bench.mjs: in ONE headless Chromium page, (1) dashu-int compiled to wasm32 (kernel/, vs_gmp_kernel.wasm),
// (2) GMP through Pyodide's gmpy2 (gmp_bench.py). Prints both tables.
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { readFileSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
const wasm = readFileSync("vs_gmp_kernel.wasm"), py = readFileSync("gmp_bench.py", "utf8");
const PAGE = `<!doctype html><meta charset="utf-8">
<script src="https://cdn.jsdelivr.net/pyodide/v314.0.7/full/pyodide.js"></script>
<script>
window.sb = async () => {
  const { instance } = await WebAssembly.instantiate(await (await fetch("/vs_gmp_kernel.wasm")).arrayBuffer(), {});
  const { setup, op } = instance.exports, names = ["mul", "div", "sqrt", "to_str", "from_str"];
  let out = "";
  for (const d of [1000, 10000, 100000, 1000000]) {
    setup(d);
    for (let k = 0; k < 5; k++) {
      let t = performance.now(); op(k, 1); const one = Math.max(performance.now() - t, 0.001);
      const reps = Math.min(Math.max(Math.ceil(200 / one), 1), 1000000);
      let best = 1e9;
      for (let i = 0; i < 3; i++) { t = performance.now(); op(k, reps); best = Math.min(best, (performance.now() - t) / 1000 / reps); }
      out += d + " " + names[k] + " " + best.toExponential(3) + "\\n";
    }
  }
  return out;
};
window.gmp = async (code) => {
  const p = await loadPyodide(); await p.loadPackage("gmpy2");
  let out = ""; p.setStdout({ batched: (s) => (out += s + "\\n") }); p.setStderr({ batched: (s) => (out += s + "\\n") });
  try { p.runPython(code); } catch (e) { out += String(e); }
  return "Pyodide " + p.version + "\\n" + out;
};
</script>`;
const server = createServer((req, res) => {
  if (req.url === "/") return res.writeHead(200, { "content-type": "text/html" }).end(PAGE);
  if (req.url === "/vs_gmp_kernel.wasm") return res.writeHead(200, { "content-type": "application/wasm" }).end(wasm);
  res.writeHead(404).end();
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const base = `http://127.0.0.1:${server.address().port}/`;
const port = 9300 + Math.floor(Math.random() * 500);
const profile = mkdtempSync(join(tmpdir(), "bb-"));
const chrome = spawn(process.env.CHROME ?? "chromium", ["--headless", "--no-sandbox", "--disable-gpu", `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, "about:blank"], { stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let targets;
for (let i = 0; i < 200; i++) { try { targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json(); break; } catch { await sleep(100); } }
const ws = new WebSocket(targets.find((t) => t.type === "page").webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let n = 0; const waiting = new Map();
ws.onmessage = (ev) => { const m = JSON.parse(ev.data); if (m.id && waiting.has(m.id)) { waiting.get(m.id)(m.result ?? m.error); waiting.delete(m.id); } };
const send = (method, params = {}) => new Promise((r) => { const id = ++n; waiting.set(id, r); ws.send(JSON.stringify({ id, method, params })); });
const evaluate = async (expression) => (await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true })).result?.value;
try {
  await send("Page.navigate", { url: base });
  for (let i = 0; i < 600; i++) { await sleep(50); if (await evaluate("typeof window.sb === 'function' && typeof loadPyodide === 'function'")) break; }
  console.log("== dashu-int, wasm32, " + (await evaluate("navigator.userAgent")));
  console.log((await evaluate("window.sb()")).trim());
  console.log("== GMP via Pyodide gmpy2");
  console.log((await evaluate(`window.gmp(${JSON.stringify(py)})`)).trim());
} finally {
  ws.close(); chrome.kill(); server.close();
  await sleep(300); rmSync(profile, { recursive: true, force: true, maxRetries: 5 });
}
