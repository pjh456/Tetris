#include "catch_amalgamated.hpp"
#include "tetris/core/engine.hpp"

using namespace tetris::core;

TEST_CASE("Engine reset determinism", "[engine]") {
    SECTION("same seed produces identical state") {
        Engine<10, 20> a, b;
        a.reset(42);
        b.reset(42);
        REQUIRE(a.state.piece == b.state.piece);
        REQUIRE(a.state.rng == b.state.rng);
        for (int i = 0; i < 5; ++i)
            REQUIRE(a.state.next[i] == b.state.next[i]);
    }

    SECTION("different seed produces different first piece") {
        Engine<10, 20> a, b;
        a.reset(42);
        b.reset(99);
        bool diff = (a.state.piece != b.state.piece) ||
                    (a.state.rng != b.state.rng);
        REQUIRE(diff);
    }
}

TEST_CASE("Engine tick and LockDelay integration", "[engine]") {
    Engine<10, 20> engine;
    engine.reset(12345);

    SECTION("tick moves piece down, not instant game over") {
        for (int i = 0; i < 100 && !engine.game_over; ++i)
            engine.tick();
        REQUIRE_FALSE(engine.game_over);
    }

    SECTION("get_lock_timer returns int") {
        int t = engine.get_lock_timer();
        REQUIRE(t >= 0);
    }

    SECTION("hard drop immediately locks and spawns next piece") {
        Piece first = engine.state.piece;
        engine.handle_action(Action::HardDrop);
        REQUIRE(engine.state.piece != first);
        REQUIRE(engine.get_lock_timer() == 0);
    }

    SECTION("multiple hard drops work without game over") {
        for (int i = 0; i < 5 && !engine.game_over; ++i)
            engine.handle_action(Action::HardDrop);
        REQUIRE_FALSE(engine.game_over);
    }
}

TEST_CASE("Engine hold mechanic", "[engine]") {
    Engine<10, 20> engine;
    engine.reset(12345);

    SECTION("first hold saves piece, sets has_hold") {
        Piece first = engine.state.piece;
        REQUIRE_FALSE(engine.has_hold);
        engine.handle_action(Action::Hold);
        REQUIRE(engine.has_hold);
        REQUIRE(engine.state.hold == first);
        REQUIRE(engine.state.hold_used);
        REQUIRE(engine.state.piece != first);
    }

    SECTION("hold after lock swaps piece back") {
        Piece first = engine.state.piece;
        engine.handle_action(Action::Hold);
        engine.handle_action(Action::HardDrop);
        engine.handle_action(Action::Hold);
        REQUIRE(engine.state.piece == first);
    }
}

TEST_CASE("Engine game over detection", "[engine]") {
    Engine<10, 20> engine;

    SECTION("spawn blocked by full board → game_over") {
        engine.reset(42);
        for (int i = 0; i < 20; ++i)
            engine.state.board.rows[i] = Board<10, 20>::FULL;
        engine.spawn();
        REQUIRE(engine.game_over);
    }

    SECTION("normal reset does not game over") {
        engine.reset(12345);
        REQUIRE_FALSE(engine.game_over);
    }
}

TEST_CASE("Engine scripted action sequence deterministic", "[engine]") {
    Engine<10, 20> a, b;
    a.reset(100);
    b.reset(100);

    Action seq[] = {
        Action::MoveLeft, Action::MoveRight, Action::RotateCW,
        Action::HardDrop, Action::Hold, Action::SoftDrop
    };

    for (auto act : seq) {
        a.handle_action(act);
        b.handle_action(act);
    }

    REQUIRE(a.state.piece == b.state.piece);
    REQUIRE(a.state.rng == b.state.rng);
    REQUIRE(a.game_over == b.game_over);
}
