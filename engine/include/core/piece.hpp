#ifndef INCLUDE_TETRIS_PIECE_HPP
#define INCLUDE_TETRIS_PIECE_HPP

#include <array>

#include "core/types.hpp"

namespace tetris
{
    struct Shape
    {
        u16 row[4]; // 每行低 4bit 有效
    };

    struct PieceDef
    {
        Shape rot[4];
    };

    constexpr u16 rot90(u16 x)
    {
        u16 r = 0;
        for (int y = 0; y < 4; ++y)
            for (int x2 = 0; x2 < 4; ++x2)
            {
                int src = y * 4 + x2;
                int dst = x2 * 4 + (3 - y);
                if (x & (1 << src))
                    r |= (1 << dst);
            }
        return r;
    }

    constexpr Shape to_rows(u16 shape)
    {
        Shape r{};
        for (int y = 0; y < 4; ++y)
        {
            u16 row = 0;
            for (int x = 0; x < 4; ++x)
            {
                if (shape & (1 << (y * 4 + x)))
                    row |= (1 << x);
            }
            r.row[y] = row;
        }
        return r;
    }

    constexpr PieceDef make_piece(u16 base)
    {
        u16 r1 = rot90(base);
        u16 r2 = rot90(r1);
        u16 r3 = rot90(r2);

        return {
            to_rows(base),
            to_rows(r1),
            to_rows(r2),
            to_rows(r3)};
    }

    constexpr auto PIECES = std::array{
        make_piece(0b0000111100000000), // I
        make_piece(0b0000011001100000), // O
        make_piece(0b0000010011100000), // T
        make_piece(0b0000001101100000), // S
        make_piece(0b0000011000110000), // Z
        make_piece(0b0000011100010000), // J
        make_piece(0b0000011101000000)  // L
    };
}

#endif // INCLUDE_TETRIS_PIECE_HPP