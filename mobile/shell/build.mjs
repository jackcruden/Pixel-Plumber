// Build the shell:
//   node build.mjs           -> dist/ (index.html + bundle.js + pp_wasm.wasm)
//   node build.mjs --single  -> also dist/single.html (everything inlined,
//                               wasm as base64 — for hosting as one file)
import { build } from "esbuild";
import { readFileSync, writeFileSync, mkdirSync, copyFileSync, existsSync } from "fs";
import { execSync } from "child_process";

const WASM = "../target/wasm32-unknown-unknown/wasm-release/pp_wasm.wasm";

if (!existsSync(WASM) || process.argv.includes("--wasm")) {
  console.log("building wasm…");
  execSync("cargo build -p pp-wasm --target wasm32-unknown-unknown --profile wasm-release", {
    cwd: "..", stdio: "inherit",
  });
}

mkdirSync("dist", { recursive: true });
await build({
  entryPoints: ["src/main.ts"],
  bundle: true,
  minify: true,
  format: "iife",
  outfile: "dist/bundle.js",
  target: "es2020",
});
copyFileSync("index.html", "dist/index.html");
copyFileSync(WASM, "dist/pp_wasm.wasm");
console.log("dist/ built");

if (process.argv.includes("--single")) {
  const html = readFileSync("index.html", "utf8");
  const js = readFileSync("dist/bundle.js", "utf8");
  const wasmB64 = readFileSync(WASM).toString("base64");
  const single = html.replace(
    '<script src="bundle.js"></script>',
    `<script>window.PP_WASM_B64=${JSON.stringify(wasmB64)};</script>\n<script>${js}</script>`,
  );
  writeFileSync("dist/single.html", single);
  console.log(`dist/single.html built (${(single.length / 1024).toFixed(0)} KB)`);
}
