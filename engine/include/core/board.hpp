#ifndef INCLUDE_TETRIS_BOARD_HPP
#define INCLUDE_TETRIS_BOARD_HPP

#include "core/types.hpp"

namespace tetris
{
    template <u8 W, u8 H>
    struct Board
    {
    public:
        static_assert(W < 64, "Board is too wide to calculate!");
        static constexpr u64 FULL = (1u << W) - 1;

        u64 rows[H] = {};

    public:
        inline bool collide(u8 y, u64 mask) const noexcept
        {
            return rows[y] & mask;
        }

        inline void place(u8 y, u64 mask) noexcept
        {
            rows[y] |= mask;
        }

        inline bool full(u8 y) const noexcept
        {
            return rows[y] == FULL;
        }

        u8 clear_lines()
        {
            int write = H - 1;
            u8 cleared = 0;

            for (int read = H - 1; read >= 0; --read)
            {
                if (rows[read] != FULL)
                    rows[write--] = rows[read];
                else
                    ++cleared;
            }

            while (write >= 0)
                rows[write--] = 0;

            return cleared;
        }
    };
}

#endif // INCLUDE_TETRIS_BOARD_HPP