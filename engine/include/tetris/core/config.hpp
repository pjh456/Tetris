#ifndef INCLUDE_TETRIS_CONFIG_HPP
#define INCLUDE_TETRIS_CONFIG_HPP

#include "tetris/core/types.hpp"

namespace tetris::core
{
    template <u16 W, u16 H>
    struct Config
    {
        static constexpr u16 width = W;
        static constexpr u16 height = H;
    };

}

#endif // INCLUDE_TETRIS_CONFIG_HPP
