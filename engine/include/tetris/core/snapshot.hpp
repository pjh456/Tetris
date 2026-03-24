#ifndef INCLUDE_TETRIS_SNAPSHOT_HPP
#define INCLUDE_TETRIS_SNAPSHOT_HPP

#include <cstring>

#include "tetris/core/state.hpp"

namespace tetris::core
{
    template <u8 W, u8 H>
    struct StateSnapshot
    {
        u64 board_rows[H];
        Piece piece;
        Rot rot;
        i8 x;
        i8 y;
        Piece hold;
        bool hold_used;
        Piece next[5];
        u8 pending_garbage;
        u32 rng;
        int combo;
        bool b2b;
        bool last_move_was_rotation;
    };

    template <u8 W, u8 H>
    inline StateSnapshot<W, H> make_snapshot(const State<W, H> &st)
    {
        StateSnapshot<W, H> snap{};
        std::memcpy(snap.board_rows, st.board.rows, sizeof(snap.board_rows));
        snap.piece = st.piece;
        snap.rot = st.rot;
        snap.x = st.x;
        snap.y = st.y;
        snap.hold = st.hold;
        snap.hold_used = st.hold_used;
        for (int i = 0; i < 5; ++i)
            snap.next[i] = st.next[i];
        snap.pending_garbage = st.pending_garbage;
        snap.rng = st.rng;
        snap.combo = st.combo;
        snap.b2b = st.b2b;
        snap.last_move_was_rotation = st.last_move_was_rotation;
        return snap;
    }

    template <u8 W, u8 H>
    inline void apply_snapshot(State<W, H> &st, const StateSnapshot<W, H> &snap)
    {
        std::memcpy(st.board.rows, snap.board_rows, sizeof(snap.board_rows));
        st.piece = snap.piece;
        st.rot = snap.rot;
        st.x = snap.x;
        st.y = snap.y;
        st.hold = snap.hold;
        st.hold_used = snap.hold_used;
        for (int i = 0; i < 5; ++i)
            st.next[i] = snap.next[i];
        st.pending_garbage = snap.pending_garbage;
        st.rng = snap.rng;
        st.combo = snap.combo;
        st.b2b = snap.b2b;
        st.last_move_was_rotation = snap.last_move_was_rotation;
    }
}

#endif // INCLUDE_TETRIS_SNAPSHOT_HPP
