#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RTC="${RTC:-cargo run -q -p rt-cli --}"
RUSTC="${RUSTC:-rustc}"
OPT="${OPT:-opt-18}"
LLI="${LLI:-lli-18}"
OUT_DIR="${OUT_DIR:-target/cross-check}"

mkdir -p "$OUT_DIR"

if [ "$#" -eq 0 ]; then
  set -- \
    examples/basic.rs \
    examples/control_flow.rs \
    examples/function_call.rs
fi

status=0

for example in "$@"; do
  base="$(basename "$example" .rs)"
  ll="$OUT_DIR/$base.ll"
  rust_src="$OUT_DIR/$base.rustc.rs"
  rust_bin="$OUT_DIR/$base.rustc.bin"

  echo "==> $example"

  $RTC -S "$example" -o "$ll"
  "$OPT" -passes=verify "$ll" -disable-output

  set +e
  "$LLI" "$ll"
  rtc_status=$?
  set -e

  {
    printf 'fn main() {\n'
    printf '    std::process::exit(rtc_main());\n'
    printf '}\n\n'
    sed '0,/fn main/s//fn rtc_main/' "$example"
  } > "$rust_src"

  "$RUSTC" --crate-name "${base}_rustc" "$rust_src" -o "$rust_bin"

  set +e
  "$rust_bin"
  rustc_status=$?
  set -e

  printf 'rtc=%s rustc=%s\n' "$rtc_status" "$rustc_status"
  if [ "$rtc_status" -ne "$rustc_status" ]; then
    echo "mismatch for $example" >&2
    status=1
  fi
done

exit "$status"
