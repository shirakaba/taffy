cmake_minimum_required(VERSION 3.13...3.26)
project(taffy_yoga_compat_tests)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT DEFINED TAFFY_YOGA_LIB_PATH)
  message(FATAL_ERROR "TAFFY_YOGA_LIB_PATH is required")
endif()

if(NOT DEFINED TAFFY_YOGA_ROOT)
  message(FATAL_ERROR "TAFFY_YOGA_ROOT is required")
endif()

if(NOT DEFINED TAFFY_COMPAT_CPP)
  message(FATAL_ERROR "TAFFY_COMPAT_CPP is required")
endif()

include(FetchContent)
include(GoogleTest)

FetchContent_Declare(
  googletest
  URL https://github.com/google/googletest/archive/refs/tags/release-1.12.1.zip
)
FetchContent_MakeAvailable(googletest)

add_library(yogacore STATIC IMPORTED GLOBAL)
set_target_properties(yogacore PROPERTIES IMPORTED_LOCATION "${TAFFY_YOGA_LIB_PATH}")
target_include_directories(yogacore INTERFACE "${TAFFY_YOGA_ROOT}")

file(GLOB_RECURSE SOURCES CONFIGURE_DEPENDS
  "${TAFFY_YOGA_ROOT}/tests/generated/*.cpp")
list(APPEND SOURCES "${TAFFY_COMPAT_CPP}")
list(APPEND SOURCES "${CMAKE_CURRENT_LIST_DIR}/AdversarialCompatTests.cpp")

add_executable(yogatests ${SOURCES})
target_link_libraries(yogatests yogacore GTest::gtest_main)
target_include_directories(yogatests PRIVATE "${TAFFY_YOGA_ROOT}" "${TAFFY_YOGA_ROOT}/tests")

enable_testing()
gtest_discover_tests(yogatests)
