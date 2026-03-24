#ifndef INCLUDE_TETRIS_BOARD_HPP
#define INCLUDE_TETRIS_BOARD_HPP

#include "tetris/core/types.hpp"

namespace tetris::core
{
    template <u8 W, u8 H>
    struct Board
    {
    public:
        static_assert(W < 64, "Board is too wide to calculate!");
        static_assert(H <= 32, "Clear mask uses 32 bits; increase mask width or reduce board height.");
        static constexpr u64 FULL = (1ULL << W) - 1;

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

        bool is_empty() const noexcept
        {
            for (int i = 0; i < H; ++i)
            {
                if (rows[i] != 0)
                    return false;
            }
            return true;
        }

    public:
        struct ClearResult
        {
            u32 mask;
            u8 count;
        };

        ClearResult clear_lines() noexcept
        {
            int write = H - 1;
            u8 cleared = 0;
            u32 mask = 0;

            for (int read = H - 1; read >= 0; --read)
            {
                if (rows[read] != FULL)
                    rows[write--] = rows[read];
                else
                {
                    ++cleared;
                    mask |= (1u << read);
                }
            }

            while (write >= 0)
                rows[write--] = 0;

            return {mask, cleared};
        }

        // 从底部插入垃圾行
        void insert_garbage(u8 lines, u8 hole_x) noexcept
        {
            if (lines == 0)
                return;

            // 1. 现有场地整体上移 (溢出部分直接丢弃/导致死亡)
            for (int y = 0; y <= H - 1 - lines; ++y)
                rows[y] = rows[y + lines];

            // 2. 在底部生成垃圾行
            // 生成一行除了 hole_x 位置为 0，其他位置为 1 的掩码
            u64 garbage_row = FULL & ~(1ULL << hole_x);

            for (int y = H - lines; y < H; ++y)
                rows[y] = garbage_row;
        }
    };
}

#endif // INCLUDE_TETRIS_BOARD_HPP
