
if(FCEUX11_BUILD_TESTS)
    add_executable(fceux11_smoke_test
        smoke_test.cpp
        git_info_stub.cpp
    )

    # Use --start-group / --end-group to resolve circular dependencies between
    # core and drivers. --allow-multiple-definition is needed because
    # fceux11_drivers_qt statically includes main.cpp which defines main().
    target_link_options(fceux11_smoke_test PRIVATE
        -Wl,--allow-multiple-definition
    )

    target_link_libraries(fceux11_smoke_test PRIVATE
        -Wl,--start-group
        fceux11_drivers_qt
        fceux11_drivers_common
        fceux11_core
        fceux11_boards
        fceux11_utils
        -Wl,--end-group
    )

    if(FCEUX11_RUST_ENABLED AND FCEUX11_RUST_STATIC_LIB)
        add_dependencies(fceux11_smoke_test fceux11_rust_build)
        target_link_libraries(fceux11_smoke_test PRIVATE
            ${FCEUX11_RUST_STATIC_LIB}
        )
    endif()

    if(MINGW)
        target_link_libraries(fceux11_smoke_test PRIVATE ntdll)
    endif()

    target_include_directories(fceux11_smoke_test PRIVATE
        ${CMAKE_SOURCE_DIR}/src
        ${CMAKE_SOURCE_DIR}/src/drivers
    )

    add_test(NAME smoke_test COMMAND fceux11_smoke_test)
endif()
