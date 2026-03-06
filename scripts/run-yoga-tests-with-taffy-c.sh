#!/usr/bin/env bash
set -euo pipefail

TAFFY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
YOGA_ROOT="${1:-/Users/jamie/git/yoga}"
BUILD_MODE="${BUILD_MODE:-debug}"

if [[ ! -d "$YOGA_ROOT/tests" ]]; then
  echo "Yoga repo not found at: $YOGA_ROOT" >&2
  exit 1
fi

pushd "$TAFFY_ROOT" >/dev/null
cargo build --manifest-path "$TAFFY_ROOT/bindings/yoga-c/Cargo.toml"
popd >/dev/null

LIB_PATH="$TAFFY_ROOT/bindings/yoga-c/target/${BUILD_MODE}/libtaffy_yoga_c.a"
if [[ ! -f "$LIB_PATH" ]]; then
  echo "ABI library missing: $LIB_PATH" >&2
  exit 1
fi

WORK_DIR="${TMPDIR:-/tmp}/taffy-yoga-c-compat"
BUILD_DIR="$WORK_DIR/build"
COMPAT_CPP="$TAFFY_ROOT/scripts/yoga-compat/CompatTestUtil.cpp"
CMAKE_SOURCE_DIR="$TAFFY_ROOT/scripts/yoga-compat"
UNSUPPORTED_LIST="$TAFFY_ROOT/scripts/yoga-compat/unsupported-tests.txt"

if [[ ! -f "$COMPAT_CPP" ]]; then
  echo "Compat source missing: $COMPAT_CPP" >&2
  exit 1
fi

if [[ ! -f "$CMAKE_SOURCE_DIR/CMakeLists.txt" ]]; then
  echo "CMake source missing: $CMAKE_SOURCE_DIR/CMakeLists.txt" >&2
  exit 1
fi

if [[ "${TAFFY_COMPAT_CLEAN:-0}" == "1" ]]; then
  rm -rf "$WORK_DIR"
fi
mkdir -p "$BUILD_DIR"

cmake -S "$CMAKE_SOURCE_DIR" -B "$BUILD_DIR" \
  -DTAFFY_YOGA_LIB_PATH="$LIB_PATH" \
  -DTAFFY_YOGA_ROOT="$YOGA_ROOT" \
  -DTAFFY_COMPAT_CPP="$COMPAT_CPP"
cmake --build "$BUILD_DIR" -j

EXCLUDE_REGEX=""
if [[ -f "$UNSUPPORTED_LIST" ]]; then
  unsupported_tests=()
  while IFS= read -r test_name; do
    unsupported_tests+=("$test_name")
  done < <(awk -F'|' '
      /^[[:space:]]*#/ { next }
      /^[[:space:]]*$/ { next }
      {
        name=$1
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", name)
        if (name != "") print name
      }
    ' "$UNSUPPORTED_LIST")

  if [[ "${#unsupported_tests[@]}" -gt 0 ]]; then
    escaped_tests=()
    for test_name in "${unsupported_tests[@]}"; do
      escaped_tests+=("$(printf '%s' "$test_name" | sed 's/[.[\*^$()+?{}\\]/\\&/g')")
    done
    EXCLUDE_REGEX="^($(IFS='|'; echo "${escaped_tests[*]}"))$"
    echo "Skipping ${#unsupported_tests[@]} unsupported Yoga tests from $UNSUPPORTED_LIST"
  fi
fi

if [[ -n "$EXCLUDE_REGEX" ]]; then
  ctest --test-dir "$BUILD_DIR" --output-on-failure -E "$EXCLUDE_REGEX"
else
  ctest --test-dir "$BUILD_DIR" --output-on-failure
fi
