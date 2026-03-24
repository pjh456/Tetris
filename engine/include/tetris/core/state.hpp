#ifndef INCLUDE_TETRIS_STATE_HPP
#define INCLUDE_TETRIS_STATE_HPP

#include "tetris/core/types.hpp"
#include "tetris/core/piece.hpp"
#include "tetris/core/board.hpp"

namespace tetris::core
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
        u32 last_clear_mask = 0;
        u8 last_clear_count = 0;
        u16 last_harddrop_cols = 0;
        i8 last_harddrop_y_min = 0;
        i8 last_harddrop_y_max = 0;
        Piece last_harddrop_piece = Piece::I;
        bool last_harddrop_valid = false;
    };
}

#endif // INCLUDE_TETRIS_STATE_HPP
