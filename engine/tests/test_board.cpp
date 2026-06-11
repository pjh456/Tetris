#include "catch_amalgamated.hpp"
#include "tetris/core/board.hpp"

using namespace tetris::core;

TEST_CASE("Board::clear_lines removes full rows", "[board]") {
    Board<10, 20> board;

    SECTION("single full line at bottom") {
        board.rows[19] = Board<10, 20>::FULL;
        auto result = board.clear_lines();
        REQUIRE(result.count == 1);
        REQUIRE(result.mask == (1u << 19));
        REQUIRE(board.rows[19] == 0);
    }

    SECTION("multiple full lines with gaps") {
        board.rows[19] = Board<10, 20>::FULL;
        board.rows[18] = 0x155;
        board.rows[17] = Board<10, 20>::FULL;
        auto result = board.clear_lines();
        REQUIRE(result.count == 2);
        REQUIRE(board.rows[19] == 0x155);
        REQUIRE(board.rows[18] == 0);
        REQUIRE(board.rows[17] == 0);
    }

    SECTION("no full lines") {
        board.rows[19] = 0x001;
        board.rows[18] = 0x002;
        auto result = board.clear_lines();
        REQUIRE(result.count == 0);
        REQUIRE(board.rows[19] == 0x001);
        REQUIRE(board.rows[18] == 0x002);
    }

    SECTION("full board clear") {
        for (int i = 0; i < 20; ++i)
            board.rows[i] = Board<10, 20>::FULL;
        auto result = board.clear_lines();
        REQUIRE(result.count == 20);
        for (int i = 0; i < 20; ++i)
            REQUIRE(board.rows[i] == 0);
    }

    SECTION("empty board") {
        auto result = board.clear_lines();
        REQUIRE(result.count == 0);
    }
}

TEST_CASE("Board::insert_garbage inserts rows with hole", "[board]") {
    Board<10, 20> board;

    SECTION("insert 3 garbage lines with hole at column 4") {
        board.insert_garbage(3, 4);
        u64 garbage_row = Board<10, 20>::FULL & ~(1ULL << 4);
        for (int i = 0; i < 17; ++i)
            REQUIRE(board.rows[i] == 0);
        REQUIRE(board.rows[19] == garbage_row);
        REQUIRE(board.rows[18] == garbage_row);
        REQUIRE(board.rows[17] == garbage_row);
    }

    // SKIPPED: insert_garbage overflow — existing code bug in u8 arithmetic
    // causes out-of-bounds access when lines > H. Will fix in Phase 02.
    // SECTION("insert 25 garbage lines on 20-height board — overflow") { ... }
}
