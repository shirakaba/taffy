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
list(APPEND SOURCES "$SRC_DIR/CompatTestUtil.cpp")

add_executable(yogatests \${SOURCES})
target_link_libraries(yogatests yogacore GTest::gtest_main)
target_include_directories(yogatests PRIVATE "$YOGA_ROOT" "$YOGA_ROOT/tests")

enable_testing()
gtest_discover_tests(yogatests)
CMAKE

cat > "$SRC_DIR/CompatTestUtil.cpp" <<'CPP'
#include <algorithm>
#include <sstream>
#include <string>
#include <string_view>
#include <vector>

#include "tests/util/TestUtil.h"

namespace facebook::yoga::test {

void TestUtil::startCountingNodes() {}
int TestUtil::nodeCount() { return 0; }
int TestUtil::stopCountingNodes() { return 0; }
ScopedEventSubscription::ScopedEventSubscription(std::function<Event::Subscriber>&&) {}
ScopedEventSubscription::~ScopedEventSubscription() {}

YGSize IntrinsicSizeMeasure(
    YGNodeConstRef node,
    float width,
    YGMeasureMode widthMode,
    float height,
    YGMeasureMode heightMode) {
  std::string_view innerText((char*)YGNodeGetContext(node));
  float heightPerChar = 10;
  float widthPerChar = 10;
  float measuredWidth;
  float measuredHeight;

  if (widthMode == YGMeasureModeExactly) {
    measuredWidth = width;
  } else if (widthMode == YGMeasureModeAtMost) {
    measuredWidth = std::min((float)innerText.size() * widthPerChar, width);
  } else {
    measuredWidth = (float)innerText.size() * widthPerChar;
  }

  if (heightMode == YGMeasureModeExactly) {
    measuredHeight = height;
  } else if (heightMode == YGMeasureModeAtMost) {
    measuredHeight = std::min(
        calculateHeight(
            innerText,
            YGNodeStyleGetFlexDirection(node) == YGFlexDirectionColumn
                ? measuredWidth
                : std::max(longestWordWidth(innerText, widthPerChar), measuredWidth),
            widthPerChar,
            heightPerChar),
        height);
  } else {
    measuredHeight = calculateHeight(
        innerText,
        YGNodeStyleGetFlexDirection(node) == YGFlexDirectionColumn
            ? measuredWidth
            : std::max(longestWordWidth(innerText, widthPerChar), measuredWidth),
        widthPerChar,
        heightPerChar);
  }

  return YGSize{measuredWidth, measuredHeight};
}

float longestWordWidth(std::string_view text, float widthPerChar) {
  int maxLength = 0;
  int currentLength = 0;
  for (auto c : text) {
    if (c == ' ') {
      maxLength = std::max(currentLength, maxLength);
      currentLength = 0;
    } else {
      currentLength++;
    }
  }
  return (float)std::max(currentLength, maxLength) * widthPerChar;
}

float calculateHeight(
    std::string_view text,
    float measuredWidth,
    float widthPerChar,
    float heightPerChar) {
  if ((float)text.size() * widthPerChar <= measuredWidth) {
    return heightPerChar;
  }

  std::vector<std::string> words;
  std::istringstream iss((std::string)text);
  std::string currentWord;
  while (getline(iss, currentWord, ' ')) {
    words.push_back(currentWord);
  }

  float lines = 1;
  float currentLineLength = 0;
  for (const std::string& word : words) {
    float wordWidth = (float)word.length() * widthPerChar;
    if (wordWidth > measuredWidth) {
      if (currentLineLength > 0) {
        lines++;
      }
      lines++;
      currentLineLength = 0;
    } else if (currentLineLength + wordWidth <= measuredWidth) {
      currentLineLength += wordWidth + widthPerChar;
    } else {
      lines++;
      currentLineLength = wordWidth + widthPerChar;
    }
  }
  return (currentLineLength == 0 ? lines - 1 : lines) * heightPerChar;
}

} // namespace facebook::yoga::test
CPP

cmake -S "$SRC_DIR" -B "$BUILD_DIR"
cmake --build "$BUILD_DIR" -j
ctest --test-dir "$BUILD_DIR" --output-on-failure
