#ifndef INCLUDE_TETRIS_RULES_HPP
#define INCLUDE_TETRIS_RULES_HPP

#include "core/piece.hpp"
#include "core/srs.hpp"
#include "core/state.hpp"

namespace tetris
{
    template <u8 W, u8 H>
    bool can_place(const State<W, H> &st, int x, int y, Rot rot)
    {
        const auto &shape = PIECES[(int)st.piece].rot[(int)rot];

        // 预先计算墙壁碰撞掩码
        u16 wall_mask = 0;
        if (x < 0)
            wall_mask |= (1 << -x) - 1; // 左墙遮挡位
        if (x + 4 > W)
            wall_mask |= (0xFFFF << std::max(0, W - x)); // 右墙遮挡位

        for (int i = 0; i < 4; i++)
        {
            u16 row = shape.row[i];
            if (!row)
                continue;

            // 墙壁碰撞检测
            if (row & wall_mask)
                return false;

            int yy = y + i;
            if (yy >= H)
                return false; // 底部越界
            if (yy < 0)
                continue; // 顶部上方允许溢出

            // 实体碰撞检测：反向平移棋盘对齐方块
            u64 b_row = st.board.rows[yy];
            u16 overlap = (x >= 0) ? (b_row >> x) : (b_row << -x);

            if (overlap & row)
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
    int hard_drop(State<W, H> &st)
    {
        int dist = 0;
        while (can_place(st, st.x, st.y + 1, st.rot))
        {
            st.y++;
            dist++;
        }
        return dist;
    }

    template <u8 W, u8 H>
    int lock_piece(State<W, H> &st)
    {
        const auto &shape =
            PIECES[(int)st.piece].rot[(int)st.rot];

        for (int i = 0; i < 4; i++)
        {
            u16 row = shape.row[i];
            if (!row)
                continue;

            int yy = st.y + i;
            if (yy < 0 || yy >= H)
                continue;

            if (st.x >= 0)
                st.board.rows[yy] |=
                    ((u64)row << st.x) & Board<W, H>::FULL;
            else
                st.board.rows[yy] |=
                    ((u64)row >> -st.x) & Board<W, H>::FULL;
        }

        return st.board.clear_lines();
    }
}

#endif // INCLUDE_TETRIS_RULES_HPP