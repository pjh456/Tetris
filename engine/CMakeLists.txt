cmake_minimum_required(VERSION 3.15)
project(tetris VERSION 0.0.1 LANGUAGES CXX)

# Export cmake compile commands
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Options
option(TETRIS_BUILD_TESTS "Build Tests" OFF)
option(TETRIS_ENABLE_OPTIMIZATION "Enable compiler optimizations" ON)

# Source files
file(GLOB_RECURSE TETRIS_SOURCE
    CONFIGURE_DEPENDS
    ${CMAKE_CURRENT_SOURCE_DIR}/src/*.cpp
)

# Library
add_library(tetris_engine ${TETRIS_SOURCE})
add_library(tetris::engine ALIAS tetris_engine)

# Header files
target_include_directories(
    tetris_engine
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Set C++ standard
target_compile_features(tetris_engine PUBLIC cxx_std_20)

# Compiler optimization
if(TETRIS_ENABLE_OPTIMIZATION)
    if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
        target_compile_options(tetris_engine PUBLIC -O3 -march=native -ffast-math)
    elseif(CMAKE_CXX_COMPILER_ID MATCHES "MSVC")
        target_compile_options(tetris_engine PUBLIC /O3 /arch:AVX2 /fp:fast)
    endif()
endif()

# Compiler options
if(MSVC)
    add_compile_options(
        /utf-8
        /MP
        /Zc:char8_t-
    )
else()
    add_link_options(
        -static-libgcc
        -static-libstdc++
        -static)
endif()

# TUI Tetris executable
add_executable(tetris src/main.cpp)
target_link_libraries(tetris
    PRIVATE
    tetris::engine
)

# Link dependency
# target_link_libraries()
if(CMAKE_PROJECT_NAME STREQUAL PROJECT_NAME)
    enable_testing()

    if(TETRIS_BUILD_TESTS)
        add_subdirectory(tests)
    endif()
endif()