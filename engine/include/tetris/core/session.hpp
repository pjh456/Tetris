#ifndef INCLUDE_TETRIS_SESSION_HPP
#define INCLUDE_TETRIS_SESSION_HPP

#include "tetris/core/engine.hpp"

namespace tetris::core
{
    template <u8 W, u8 H>
    class GameSession
    {
    private:
        Engine<W, H> m_engine;

    public:
        void reset(u32 seed = 12345) { m_engine.reset(seed); }
        AttackResult handle_action(Action act) { return m_engine.handle_action(act); }
        AttackResult tick() { return m_engine.tick(); }

        bool is_game_over() const { return m_engine.game_over; }

        State<W, H> &state() { return m_engine.state; }
        const State<W, H> &state() const { return m_engine.state; }
    };
}

#endif // INCLUDE_TETRIS_SESSION_HPP
