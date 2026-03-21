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

    // 4x4 矩阵顺时针旋转 (用于 I 和 O)
    constexpr u16 rot90_4x4(u16 x)
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

    // 3x3 矩阵顺时针旋转 (用于 J, L, S, T, Z)
    constexpr u16 rot90_3x3(u16 x)
    {
        u16 r = 0;
        for (int y = 0; y < 3; ++y)
            for (int x2 = 0; x2 < 3; ++x2)
            {
                int src = y * 4 + x2;
                int dst = x2 * 4 + (2 - y); // 3x3 中心旋转
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

    constexpr PieceDef make_piece_4x4(u16 base)
    {
        u16 r1 = rot90_4x4(base);
        u16 r2 = rot90_4x4(r1);
        u16 r3 = rot90_4x4(r2);

        return {
            to_rows(base),
            to_rows(r1),
            to_rows(r2),
            to_rows(r3)};
    }

    constexpr PieceDef make_piece_3x3(u16 base)
    {
        u16 r1 = rot90_3x3(base);
        u16 r2 = rot90_3x3(r1);
        u16 r3 = rot90_3x3(r2);

        return {
            to_rows(base),
            to_rows(r1),
            to_rows(r2),
            to_rows(r3)};
    }

    constexpr auto PIECES = std::array{
        make_piece_4x4(0x00F0), // I: 0000 0000 1111 0000 (y=1 处四个相连)
        make_piece_4x4(0x0660), // O: 0000 0110 0110 0000 (居中)
        make_piece_3x3(0x0072), // T: 0000 0000 0111 0010 (尖端朝上)
        make_piece_3x3(0x0036), // S: 0000 0000 0011 0110
        make_piece_3x3(0x0063), // Z: 0000 0000 0110 0011
        make_piece_3x3(0x0071), // J: 0000 0000 0111 0001
        make_piece_3x3(0x0074)  // L: 0000 0000 0111 0100
    };
}

#endif // INCLUDE_TETRIS_PIECE_HPP