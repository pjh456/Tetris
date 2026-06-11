#include "catch_amalgamated.hpp"
#include "tetris/core/attack.hpp"
#include "tetris/core/board.hpp"
#include "tetris/core/piece.hpp"
#include "tetris/core/state.hpp"

using namespace tetris::core;

TEST_CASE("calculate_attack normal damage values", "[attack]") {
    State<10, 20> st{};
    st.board.rows[0] = 1; // prevent perfect clear

    SECTION("0 lines cleared returns zero") {
        auto res = calculate_attack(st, 0);
        REQUIRE(res.damage == 0);
        REQUIRE(st.combo == 0);
    }

    SECTION("1 line = 0 damage") {
        auto res = calculate_attack(st, 1);
        REQUIRE(res.damage == 0);
    }

    SECTION("2 lines = 1 damage") {
        auto res = calculate_attack(st, 2);
        REQUIRE(res.damage == 1);
    }

    SECTION("3 lines = 2 damage") {
        auto res = calculate_attack(st, 3);
        REQUIRE(res.damage == 2);
    }

    SECTION("4 lines / Tetris = 4 damage, sets B2B") {
        auto res = calculate_attack(st, 4);
        REQUIRE(res.damage == 4);
        REQUIRE(res.is_b2b == false);
        REQUIRE(st.b2b == true);
    }
}

TEST_CASE("calculate_attack T-Spin damage", "[attack]") {
    State<10, 20> st{};
    st.piece = Piece::T;
    st.last_move_was_rotation = true;

    // T-piece center at (x+1, y+1). Block 3 corners to trigger T-Spin.
    int cx = st.x + 1;
    int cy = st.y + 1;

    // Mark 3 of 4 corners as occupied via board cells
    auto mark_3_corners = [&]() {
        st.board.rows[cy - 1] |= (1ULL << (cx - 1)); // top-left
        st.board.rows[cy - 1] |= (1ULL << (cx + 1)); // top-right
        st.board.rows[cy + 1] |= (1ULL << (cx + 1)); // bottom-right
        // bottom-left left unoccupied → 3 corners = T-Spin
    };

    SECTION("T-Spin single = 2 damage") {
        mark_3_corners();
        auto res = calculate_attack(st, 1);
        REQUIRE(res.is_tspin == true);
        REQUIRE(res.damage == 2);
    }

    SECTION("T-Spin double = 4 damage") {
        mark_3_corners();
        auto res = calculate_attack(st, 2);
        REQUIRE(res.is_tspin == true);
        REQUIRE(res.damage == 4);
    }

    SECTION("T-Spin triple = 6 damage") {
        mark_3_corners();
        auto res = calculate_attack(st, 3);
        REQUIRE(res.is_tspin == true);
        REQUIRE(res.damage == 6);
    }

    SECTION("not a T-Spin without last_move_was_rotation") {
        mark_3_corners();
        st.last_move_was_rotation = false;
        auto res = calculate_attack(st, 2);
        REQUIRE(res.is_tspin == false);
    }
}

TEST_CASE("calculate_attack B2B tracking", "[attack]") {
    State<10, 20> st{};
    st.board.rows[0] = 1; // prevent perfect clear

    SECTION("first Tetris sets B2B, not yet consecutive") {
        auto res = calculate_attack(st, 4);
        REQUIRE(res.is_b2b == false);
        REQUIRE(st.b2b == true);
        REQUIRE(res.damage == 4);
    }

    SECTION("consecutive Tetris gets B2B bonus +1") {
        st.b2b = true;
        auto res = calculate_attack(st, 4);
        REQUIRE(res.is_b2b == true);
        REQUIRE(st.b2b == true);
        REQUIRE(res.damage == 5);
    }

    SECTION("non-Tetris clear breaks B2B") {
        st.b2b = true;
        auto res = calculate_attack(st, 2);
        REQUIRE(res.is_b2b == false);
        REQUIRE(st.b2b == false);
        REQUIRE(res.damage == 1);
    }

    SECTION("T-Spin sets B2B") {
        st.piece = Piece::T;
        st.last_move_was_rotation = true;
        int cx = st.x + 1;
        int cy = st.y + 1;
        st.board.rows[cy - 1] |= (1ULL << (cx - 1));
        st.board.rows[cy - 1] |= (1ULL << (cx + 1));
        st.board.rows[cy + 1] |= (1ULL << (cx + 1));
        auto res = calculate_attack(st, 2);
        REQUIRE(res.is_tspin == true);
        REQUIRE(st.b2b == true);
    }
}

TEST_CASE("calculate_attack combo scaling", "[attack]") {
    State<10, 20> st{};
    st.board.rows[0] = 1; // prevent perfect clear

    SECTION("combo 0: 1 line = 0 damage") {
        st.combo = 0;
        auto res = calculate_attack(st, 1);
        REQUIRE(res.damage == 0);
        REQUIRE(st.combo == 1);
    }

    SECTION("combo 1: 1 line = 0 damage") {
        st.combo = 1;
        auto res = calculate_attack(st, 1);
        REQUIRE(res.damage == 0);
        REQUIRE(st.combo == 2);
    }

    SECTION("combo 2: 1 line = 1 damage") {
        st.combo = 2;
        auto res = calculate_attack(st, 1);
        REQUIRE(res.damage == 1);
        REQUIRE(st.combo == 3);
    }

    SECTION("combo 3: 1 line = 1 damage") {
        st.combo = 3;
        auto res = calculate_attack(st, 1);
        REQUIRE(res.damage == 1);
        REQUIRE(st.combo == 4);
    }
}

TEST_CASE("calculate_attack perfect clear", "[attack]") {
    State<10, 20> st{};

    SECTION("empty board after clear = perfect clear + 10") {
        auto res = calculate_attack(st, 4);
        REQUIRE(res.perfect_clear == true);
        REQUIRE(res.damage == 14);
    }

    SECTION("non-empty board = no perfect clear") {
        st.board.rows[19] = 0x001;
        auto res = calculate_attack(st, 1);
        REQUIRE(res.perfect_clear == false);
        REQUIRE(res.damage == 0);
    }
}
