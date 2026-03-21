#ifndef INCLUDE_TETRIS_RULES_HPP
#define INCLUDE_TETRIS_RULES_HPP

#include "core/piece.hpp"
#include "core/srs.hpp"
#include "core/state.hpp"

namespace tetris
{
    struct StepResult
    {
        int lines;
        int attack;
        bool game_over;
    };

    template <u8 W, u8 H>
    bool can_place(
        const State<W, H> &st,
        int x, int y, Rot rot)
    {
        const auto &shape =
            PIECES[(int)st.piece].rot[(int)rot];

        constexpr u16 FULL = (1u << W) - 1;

        for (int i = 0; i < 4; i++)
        {
            u16 row = shape.row[i];
            if (!row)
                continue; // 跳过空行

            int yy = y + i;

            // 底部越界 → 失败
            if (yy >= H)
                return false;

            // 顶部溢出 → 允许（跳过碰撞检测）
            if (yy < 0)
                continue;

            // ===== 横向处理=====
            u16 shifted;

            if (x >= 0)
            {
                // 右移进棋盘
                if (x >= W)
                    return false; // 完全在右边外
                shifted = row << x;

                // 右越界检测
                if (shifted & ~FULL)
                    return false;
            }
            else
            {
                // 左越界，需要右移 row
                int sx = -x;

                if (sx >= 4)
                    return false; // 整块在左边外
                if (row & ((1u << (-x)) - 1))
                    return false; // 检测是否有被截断的 bit

                shifted = row >> sx;
            }

            // ===== 碰撞检测 =====
            if (st.board.rows[yy] & shifted)
                return false;
        }

        return true;
    }

    template <u8 W, u8 H>
    bool try_rotate(State<W, H> &st, Rot to)
    {
        auto &kicks =
            SRS.data[(int)st.piece][(int)st.rot][(int)to];

        for (int i = 0; i < 5; i++)
        {
            int nx = st.x + kicks[i].dx;
            int ny = st.y - kicks[i].dy;

            if (can_place(st, nx, ny, to))
            {
                st.x = nx;
                st.y = ny;
                st.rot = to;
                return true;
            }
        }
        return false;
    }

    template <u8 W, u8 H>
    bool try_move(State<W, H> &st, int dx, int dy)
    {
        if (can_place(st, st.x + dx, st.y + dy, st.rot))
        {
            st.x += dx;
            st.y += dy;
            return true;
        }
        return false;
    }

    template <u8 W, u8 H>
    void hard_drop(State<W, H> &st)
    {
        while (can_place(
            st,
            st.x, st.y + 1, st.rot))
            st.y++;
    }

    template <u8 W, u8 H>
    int lock_piece(State<W, H> &st)
    {
        const auto &shape =
            PIECES[(int)st.piece].rot[(int)st.rot];

        constexpr u16 FULL = (1u << W) - 1;

        for (int i = 0; i < 4; i++)
        {
            u16 row = shape.row[i];
            if (!row)
                continue;

            int yy = st.y + i;
            if (yy < 0 || yy >= H)
                continue;

            u16 shifted;

            if (st.x >= 0)
            {
                if (st.x >= W)
                    continue;
                shifted = row << st.x;

                // 裁剪掉越界部分
                shifted &= FULL;
            }
            else
            {
                int sx = -st.x;
                if (sx >= 4)
                    continue;
                shifted = row >> sx;
            }

            st.board.rows[yy] |= shifted;
        }

        return st.board.clear_lines();
    }
}

#endif // INCLUDE_TETRIS_RULES_HPP