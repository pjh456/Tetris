#ifndef INCLUDE_TETRIS_SESSION_HPP
#define INCLUDE_TETRIS_SESSION_HPP

#include "tetris/core/engine.hpp"

namespace tetris::core
{
    template <u8 W, u8 H>
    class GameSession
    {
    private:
        Engine<W, H> engine_;

    public:
        void reset(u32 seed = 12345) { engine_.reset(seed); }
        AttackResult handle_action(Action act) { return engine_.handle_action(act); }
        AttackResult tick() { return engine_.tick(); }

        bool is_game_over() const { return engine_.game_over; }

        State<W, H> &state() { return engine_.state; }
        const State<W, H> &state() const { return engine_.state; }
    };
}

#endif // INCLUDE_TETRIS_SESSION_HPP
