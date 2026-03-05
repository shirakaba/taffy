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
SRC_DIR="$WORK_DIR/src"
BUILD_DIR="$WORK_DIR/build"
COMPAT_CPP="$TAFFY_ROOT/scripts/yoga-compat/CompatTestUtil.cpp"

if [[ ! -f "$COMPAT_CPP" ]]; then
  echo "Compat source missing: $COMPAT_CPP" >&2
  exit 1
fi

rm -rf "$WORK_DIR"
mkdir -p "$SRC_DIR" "$BUILD_DIR"

cat > "$SRC_DIR/CMakeLists.txt" <<CMAKE
cmake_minimum_required(VERSION 3.13...3.26)
project(taffy_yoga_compat_tests)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(FetchContent)
include(GoogleTest)

FetchContent_Declare(
  googletest
  URL https://github.com/google/googletest/archive/refs/tags/release-1.12.1.zip
)
FetchContent_MakeAvailable(googletest)

add_library(yogacore STATIC IMPORTED GLOBAL)
set_target_properties(yogacore PROPERTIES IMPORTED_LOCATION "$LIB_PATH")
target_include_directories(yogacore INTERFACE "$YOGA_ROOT")

file(GLOB_RECURSE SOURCES CONFIGURE_DEPENDS
    "$YOGA_ROOT/tests/generated/*.cpp")
list(APPEND SOURCES "$COMPAT_CPP")

add_executable(yogatests \${SOURCES})
target_link_libraries(yogatests yogacore GTest::gtest_main)
target_include_directories(yogatests PRIVATE "$YOGA_ROOT" "$YOGA_ROOT/tests")

enable_testing()
gtest_discover_tests(yogatests)
CMAKE

cmake -S "$SRC_DIR" -B "$BUILD_DIR"
cmake --build "$BUILD_DIR" -j
ctest --test-dir "$BUILD_DIR" --output-on-failure
