#!/bin/sh
# Check that the library builds for WebAssembly and converts there the
# same as natively.
#
# Usage: tools/check_wasm.sh [file.indd...]
#
# 1. Builds the library for wasm32-unknown-unknown. The `indd` binary is
#    not built: it reads and writes files.
# 2. Builds a probe module: a cdylib, generated under the target
#    directory, that exports a function calling `indd::convert`. Node.js
#    checks that the module imports nothing, so a conversion needs no
#    file system, clock or other host function.
# 3. Converts each file given, or else each fetched fixture, in the probe
#    module and compares the package byte for byte with the output of the
#    native `indd convert`.
#
# Needs the target (`rustup target add wasm32-unknown-unknown`). Steps 2
# and 3 need Node.js and are skipped without it. Exits 1 if a check fails.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
target=${CARGO_TARGET_DIR:-$root/target}
case $target in /*) ;; *) target=$root/$target ;; esac
export CARGO_TARGET_DIR="$target"

if ! rustup target list --installed | grep -qx wasm32-unknown-unknown; then
    echo "missing target: rustup target add wasm32-unknown-unknown" >&2
    exit 1
fi

echo "library: cargo build --lib --target wasm32-unknown-unknown" >&2
cargo build -q --release --lib --target wasm32-unknown-unknown --manifest-path "$root/Cargo.toml"

if ! command -v node >/dev/null 2>&1; then
    echo "node not found; skipping the probe module" >&2
    exit 0
fi

work=$target/wasm-check
probe=$work/probe
mkdir -p "$probe/src" "$work/out"
cat >"$probe/Cargo.toml" <<EOF
[package]
name = "indd-wasm-probe"
version = "0.0.0"
edition = "2024"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
indd = { path = "$root" }

[workspace]
EOF
cat >"$probe/src/lib.rs" <<'EOF'
//! Probe for tools/check_wasm.sh: `indd::convert` for a WebAssembly host.

use std::sync::Mutex;

static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Room for `len` bytes, for the host to fill and pass to `convert`.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    std::mem::ManuallyDrop::new(Vec::<u8>::with_capacity(len)).as_mut_ptr()
}

/// Convert the `len` bytes at `input`, with the document name in the
/// `name_len` bytes at `name` (both from `alloc`). Returns the length of
/// the package, which `output` points to, or -1 if the conversion fails.
///
/// # Safety
///
/// Both buffers come from `alloc` with these lengths and are filled.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn convert(input: *mut u8, len: usize, name: *mut u8, name_len: usize) -> isize {
    let input = unsafe { Vec::from_raw_parts(input, len, len) };
    let name = unsafe { Vec::from_raw_parts(name, name_len, name_len) };
    let (Ok(c), Ok(mut out)) = (indd::convert(&input, &String::from_utf8_lossy(&name)), OUTPUT.lock()) else {
        return -1;
    };
    *out = c.idml;
    out.len() as isize
}

/// The package of the last conversion.
#[unsafe(no_mangle)]
pub extern "C" fn output() -> *const u8 {
    OUTPUT.lock().map_or(std::ptr::null(), |o| o.as_ptr())
}
EOF
cat >"$work/run.mjs" <<'EOF'
// Usage: node run.mjs probe.wasm [input name expected]...
// `expected` is the native package, or "-" if the native conversion failed.
import { readFileSync } from "node:fs";

const [wasm, ...args] = process.argv.slice(2);
const module = new WebAssembly.Module(readFileSync(wasm));
const imports = WebAssembly.Module.imports(module);
if (imports.length > 0) {
  console.log(`probe module imports: ${imports.map((i) => `${i.module}.${i.name}`).join(" ")}`);
  process.exit(1);
}
const { exports } = new WebAssembly.Instance(module, {});
const put = (bytes) => {
  const at = exports.alloc(bytes.length);
  new Uint8Array(exports.memory.buffer, at, bytes.length).set(bytes);
  return at;
};
let failed = 0;
for (let i = 0; i < args.length; i += 3) {
  const [input, name, expected] = args.slice(i, i + 3);
  const bytes = readFileSync(input);
  const label = new TextEncoder().encode(name);
  const n = exports.convert(put(bytes), bytes.length, put(label), label.length);
  let result;
  if (n < 0) {
    result = expected === "-" ? "same (both fail)" : "differs: fails in WebAssembly only";
  } else if (expected === "-") {
    result = "differs: fails natively only";
  } else {
    const out = Buffer.from(new Uint8Array(exports.memory.buffer, exports.output(), n));
    result = out.equals(readFileSync(expected)) ? `same (${n} bytes)` : "differs";
  }
  if (result.startsWith("differs")) failed++;
  console.log(`${input}: ${result}`);
}
console.log(`${args.length / 3} files, ${failed} differ; no imports`);
process.exit(failed > 0 ? 1 : 0);
EOF

echo "probe module: cargo build --target wasm32-unknown-unknown" >&2
cargo build -q --release --target wasm32-unknown-unknown --manifest-path "$probe/Cargo.toml"
cargo build -q --release --bin indd --manifest-path "$root/Cargo.toml"

if [ $# -eq 0 ]; then
    for f in "$root"/tests/fixtures/files/*/*.indd "$root"/tests/fixtures/files/*/*.indt; do
        [ -f "$f" ] && set -- "$@" "$f"
    done
    if [ $# -eq 0 ]; then
        echo "no files given and no fixtures fetched; checking imports only" >&2
    fi
fi
i=0
for f; do
    shift
    i=$((i + 1))
    expected=$work/out/$i.idml
    if ! "$target/release/indd" convert "$f" "$expected" 2>/dev/null; then
        expected=-
    fi
    set -- "$@" "$f" "$(basename "$f")" "$expected"
done
node "$work/run.mjs" "$target/wasm32-unknown-unknown/release/indd_wasm_probe.wasm" "$@"
