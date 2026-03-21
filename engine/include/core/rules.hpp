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

        for (int i = 0; i < 4; i++)
        {
            int yy = y + i;
            if (yy < 0 || yy >= H)
                return false;

            u16 mask = shape.row[i] << x;

            if (mask & ~((1u << W) - 1))
                return false;

            if (st.board.rows[yy] & mask)
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

        for (int i = 0; i < 4; i++)
        {
            int yy = st.y + i;
            if (yy < 0 || yy >= H)
                continue;

            u16 mask = shape.row[i] << st.x;
            st.board.rows[yy] |= mask;
        }

        return st.board.clear_lines();
    }
}

#endif // INCLUDE_TETRIS_RULES_HPP