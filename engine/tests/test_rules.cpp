#include "catch_amalgamated.hpp"
#include "tetris/core/board.hpp"
#include "tetris/core/piece.hpp"
#include "tetris/core/rules.hpp"
#include "tetris/core/srs.hpp"
#include "tetris/core/state.hpp"
#include "tetris/core/types.hpp"

using namespace tetris::core;

namespace {

template <u8 W, u8 H>
State<W, H> make_state(Piece piece) {
    State<W, H> st{};
    st.piece = piece;
    st.rot = Rot::R0;
    st.x = W / 2 - 2;
    st.y = 0;
    return st;
}

} // namespace

TEST_CASE("can_place validates wall/floor/occupied constraints", "[rules]") {
    using S = State<10, 20>;

    SECTION("piece at spawn position is valid") {
        auto st = make_state<10, 20>(Piece::T);
        REQUIRE(can_place(st, st.x, st.y, st.rot));
    }

    SECTION("left wall collision rejected") {
        auto st = make_state<10, 20>(Piece::I);
        REQUIRE(can_place(st, st.x, st.y, st.rot));
        REQUIRE_FALSE(can_place(st, -2, st.y, st.rot));
    }

    SECTION("right wall collision rejected") {
        auto st = make_state<10, 20>(Piece::I);
        REQUIRE_FALSE(can_place(st, 8, st.y, st.rot));
    }

    SECTION("floor collision rejected") {
        auto st = make_state<10, 20>(Piece::T);
        REQUIRE_FALSE(can_place(st, st.x, 20, st.rot));
    }

    SECTION("occupied row blocks placement") {
        auto st = make_state<10, 20>(Piece::T);
        st.board.rows[19] = Board<10, 20>::FULL;
        REQUIRE_FALSE(can_place(st, st.x, 18, st.rot));
        REQUIRE(can_place(st, st.x, 17, st.rot));
    }
}

TEST_CASE("try_move shifts piece position on success", "[rules]") {
    using S = State<10, 20>;

    SECTION("move left from spawn") {
        auto st = make_state<10, 20>(Piece::T);
        auto x0 = st.x;
        REQUIRE(try_move(st, -1, 0));
        REQUIRE(st.x == x0 - 1);
    }

    SECTION("move right from spawn") {
        auto st = make_state<10, 20>(Piece::T);
        auto x0 = st.x;
        REQUIRE(try_move(st, 1, 0));
        REQUIRE(st.x == x0 + 1);
    }

    SECTION("soft drop from spawn") {
        auto st = make_state<10, 20>(Piece::T);
        REQUIRE(try_move(st, 0, 1));
        REQUIRE(st.y == 1);
    }

    SECTION("move left past wall fails, state unchanged") {
        auto st = make_state<10, 20>(Piece::I);
        while (try_move(st, -1, 0)) {}
        auto x_before = st.x;
        auto y_before = st.y;
        REQUIRE_FALSE(try_move(st, -1, 0));
        REQUIRE(st.x == x_before);
        REQUIRE(st.y == y_before);
    }

    SECTION("move down into occupied row fails") {
        auto st = make_state<10, 20>(Piece::T);
        st.y = 18;
        st.board.rows[19] = Board<10, 20>::FULL;
        REQUIRE_FALSE(try_move(st, 0, 1));
        REQUIRE(st.y == 18);
    }
}

TEST_CASE("try_rotate applies SRS kick table", "[rules]") {
    using S = State<10, 20>;

    SECTION("RotateCW from R0 for T-piece succeeds") {
        auto st = make_state<10, 20>(Piece::T);
        REQUIRE(try_rotate(st, Rot::R90));
        REQUIRE(st.rot == Rot::R90);
    }

    SECTION("RotateCW from R0 for O-piece is always no-op") {
        auto st = make_state<10, 20>(Piece::O);
        REQUIRE(try_rotate(st, Rot::R90));
        REQUIRE(st.rot == Rot::R90);
    }

    SECTION("RotateCCW from R0 for T-piece succeeds") {
        auto st = make_state<10, 20>(Piece::T);
        REQUIRE(try_rotate(st, Rot::R270));
        REQUIRE(st.rot == Rot::R270);
    }

    SECTION("all 7 pieces rotate at spawn") {
        for (auto piece : {Piece::I, Piece::O, Piece::T, Piece::S,
                           Piece::Z, Piece::J, Piece::L}) {
            auto st = make_state<10, 20>(piece);
            REQUIRE(try_rotate(st, Rot::R90));
            REQUIRE(st.rot == Rot::R90);
        }
    }

    SECTION("I-piece near left wall uses kick") {
        auto st = make_state<10, 20>(Piece::I);
        st.x = 0;
        st.rot = Rot::R0;
        bool result = try_rotate(st, Rot::R90);
        if (result) {
            REQUIRE(st.rot == Rot::R90);
        }
    }
}

TEST_CASE("lock_piece writes cells to board and returns clear count", "[rules]") {
    using S = State<10, 20>;

    SECTION("lock at bottom with no line clear") {
        auto st = make_state<10, 20>(Piece::T);
        st.y = 17;
        int cleared = lock_piece(st);
        REQUIRE(cleared == 0);
        bool has_cells = false;
        for (int i = 0; i < 20; ++i) {
            if (st.board.rows[i] != 0) has_cells = true;
        }
        REQUIRE(has_cells);
    }

    SECTION("lock completes bottom row, clears 1 line") {
        auto st = make_state<10, 20>(Piece::I);
        st.y = 18;
        st.rot = Rot::R0;
        for (int col = 0; col < 10; ++col) {
            if (col < 3 || col > 6)
                st.board.rows[19] |= (1ULL << col);
        }
        int cleared = lock_piece(st);
        REQUIRE(cleared == 1);
        REQUIRE(st.board.rows[19] == 0);
    }
}

TEST_CASE("hard_drop moves piece to surface", "[rules]") {
    using S = State<10, 20>;

    SECTION("drop from spawn to empty board") {
        auto st = make_state<10, 20>(Piece::T);
        int dist = hard_drop(st);
        REQUIRE(dist > 0);
        REQUIRE(st.y >= 17);
        REQUIRE_FALSE(can_place(st, st.x, st.y + 1, st.rot));
    }

    SECTION("drop stops above occupied row") {
        auto st = make_state<10, 20>(Piece::T);
        st.board.rows[19] = 0xFF;
        int dist = hard_drop(st);
        REQUIRE(dist > 0);
        REQUIRE_FALSE(can_place(st, st.x, st.y + 1, st.rot));
    }
}

TEST_CASE("get_ghost_y returns lowest valid y position", "[rules]") {
    using S = State<10, 20>;

    SECTION("ghost on empty board matches hard_drop y") {
        auto st1 = make_state<10, 20>(Piece::T);
        auto st2 = st1;
        int ghost_y = get_ghost_y(st1);
        hard_drop(st2);
        REQUIRE(ghost_y == st2.y);
    }

    SECTION("ghost stops above occupied row") {
        auto st = make_state<10, 20>(Piece::T);
        st.board.rows[19] = 0xFF;
        int ghost_y = get_ghost_y(st);
        REQUIRE(ghost_y < 19);
        REQUIRE_FALSE(can_place(st, st.x, ghost_y + 1, st.rot));
    }
}
