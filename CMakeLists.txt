
cmake_minimum_required(VERSION 3.28)

project(FCEUX11 VERSION 0.2.0 LANGUAGES CXX C)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

option(FCEUX11_BUILD_TESTS "Build smoke and regression tests" ON)

add_subdirectory( src )

if(FCEUX11_BUILD_TESTS)
    enable_testing()
endif()

