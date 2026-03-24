#ifndef INCLUDE_TETRIS_TYPES_HPP
#define INCLUDE_TETRIS_TYPES_HPP

#include <cstdint>

namespace tetris::core
{
    using i8 = int8_t;
    using u8 = uint8_t;
    using u16 = uint16_t;
    using u32 = uint32_t;
    using u64 = uint64_t;

    enum class Piece : u8
    {
        I,
        O,
        T,
        S,
        Z,
        J,
        L
    };

    enum class Rot : u8
    {
        R0,
        R90,
        R180,
        R270
    };

    struct Vec2
    {
        i8 x, y;
    };
}

#endif // INCLUDE_TETRIS_TYPES_HPP
