
# Shared link configuration for test executables
function(fceux11_add_test_executable target_name)
    add_executable(${target_name} ${ARGN})

    target_link_libraries(${target_name} PRIVATE
        fceux11_drivers_qt
        fceux11_drivers_common
        fceux11_core
        fceux11_boards
        fceux11_utils
    )

    if(FCEUX11_RUST_ENABLED AND FCEUX11_RUST_STATIC_LIB)
        add_dependencies(${target_name} fceux11_rust_build)
        target_link_libraries(${target_name} PRIVATE
            ${FCEUX11_RUST_STATIC_LIB}
        )
    endif()

    if(WIN32)
        target_link_libraries(${target_name} PRIVATE ntdll)
    endif()

    target_include_directories(${target_name} PRIVATE
        ${CMAKE_SOURCE_DIR}/src
        ${CMAKE_SOURCE_DIR}/src/drivers
    )
endfunction()

if(FCEUX11_BUILD_TESTS)
    # Smoke test (Task 1.0)
    fceux11_add_test_executable(fceux11_smoke_test
        smoke_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME smoke_test COMMAND fceux11_smoke_test)

    # Mapper regression tests (Task 1.1)
    fceux11_add_test_executable(fceux11_mapper_load_test
        boards/mapper_load_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME mapper_load_test
             COMMAND fceux11_mapper_load_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

    fceux11_add_test_executable(fceux11_mapper_reset_test
        boards/mapper_reset_test.cpp
        git_info_stub.cpp
    )
    add_test(NAME mapper_reset_test
             COMMAND fceux11_mapper_reset_test
             WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})
endif()
