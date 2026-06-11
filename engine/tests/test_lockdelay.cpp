#include "catch_amalgamated.hpp"
#include "tetris/core/lockdelay.hpp"

using namespace tetris::core;

TEST_CASE("LockDelay basic behavior", "[lockdelay]") {
    LockDelay ld;

    SECTION("not active by default") {
        REQUIRE(!ld.active);
        REQUIRE(ld.move_reset_count == 0);
        REQUIRE(!ld.update());
        REQUIRE(ld.remaining_ms() == 0);
    }

    SECTION("start activates timer") {
        ld.start();
        REQUIRE(ld.active);
        REQUIRE(ld.remaining_ms() > 0);
        REQUIRE(ld.remaining_ms() <= LockDelay::LOCK_DELAY_MS);
        REQUIRE(!ld.update());
    }

    SECTION("start is idempotent when already active") {
        ld.start();
        int first_ms = ld.remaining_ms();
        ld.start();
        REQUIRE(ld.active);
    }

    SECTION("reset increments move counter") {
        ld.start();
        ld.reset();
        REQUIRE(ld.move_reset_count == 1);
        REQUIRE(ld.active);
    }

    SECTION("15 resets reached, timer still runs") {
        ld.start();
        for (int i = 0; i < 15; i++)
            ld.reset();
        REQUIRE(ld.move_reset_count == 15);
        REQUIRE(!ld.update());
        REQUIRE(ld.active);
    }

    SECTION("cancel clears state") {
        ld.start();
        ld.cancel();
        REQUIRE(!ld.active);
        REQUIRE(ld.move_reset_count == 0);
        REQUIRE(!ld.update());
        REQUIRE(ld.remaining_ms() == 0);
    }

    SECTION("reset activates inactive LockDelay") {
        REQUIRE(!ld.active);
        ld.reset();
        REQUIRE(ld.active);
        REQUIRE(ld.move_reset_count == 1);
        REQUIRE(ld.remaining_ms() > 0);
    }
}
