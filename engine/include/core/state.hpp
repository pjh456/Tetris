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

        u8 pending_garbage = 0;              // 正在排队等待进入场地的垃圾行数
        bool last_move_was_rotation = false; // 判定 T-Spin 核心标记
    };
}

#endif // INCLUDE_TETRIS_STATE_HPP