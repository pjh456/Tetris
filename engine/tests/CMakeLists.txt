file(GLOB TETRIS_TEST_SOURCES
    CONFIGURE_DEPENDS
    *.cpp)

foreach(test_file ${TETRIS_TEST_SOURCES})
    get_filename_component(test_name ${test_file} NAME_WE)

    add_executable(${test_name} ${test_file})

    target_link_libraries(${test_name}
        PRIVATE
        tetris::engine)

    add_test(
        NAME ${test_name}
        COMMAND ${test_name}
    )
endforeach()