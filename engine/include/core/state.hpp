#ifndef INCLUDE_TETRIS_STATE_HPP
#define INCLUDE_TETRIS_STATE_HPP

#include "core/types.hpp"
#include "core/piece.hpp"
#include "core/board.hpp"

namespace tetris
{
    template <u8 W, u8 H>
    struct State
    {
        Board<W, H> board;

        Piece piece;
        Rot rot;
        i8 x, y;

        Piece hold;
        bool hold_used;

        Piece next[5];

        u32 rng;
        int combo;
        bool b2b;
    };
}

#endif // INCLUDE_TETRIS_STATE_HPP