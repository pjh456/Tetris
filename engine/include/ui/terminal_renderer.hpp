#ifndef INCLUDE_TETRIS_TERMINAL_RENDERER_HPP
#define INCLUDE_TETRIS_TERMINAL_RENDERER_HPP

#include <iostream>
#include <string>

#include "core/state.hpp"
#include "core/rules.hpp"
#include "core/piece.hpp"

namespace tetris::ui
{
    class TerminalRenderer
    {
    public:
        void move_cursor(int x, int y) const
        {
            std::cout << "\033[" << y << ";" << x << "H";
        }

        void clear_screen() const
        {
            std::cout << "\033[2J\033[H";
        }

        template <u8 W, u8 H>
        void render_board(const State<W, H> &st, int offset_x, int offset_y, const std::string &title) const
        {
            move_cursor(offset_x, offset_y);
            std::cout << "=== " << title << " ===";

            int ghost_y = get_ghost_y(st);
            const auto &shape = PIECES[(int)st.piece].rot[(int)st.rot];

            const char *colors[] = {
                "\033[0m", "\033[46m", "\033[43m", "\033[45m",
                "\033[42m", "\033[41m", "\033[44m", "\033[47m"};

            for (int y = 0; y < H; ++y)
            {
                move_cursor(offset_x, offset_y + 1 + y);
                std::cout << "<!";

                for (int x = 0; x < W; ++x)
                {
                    bool is_active = false;
                    bool is_ghost = false;

                    if (y >= st.y && y < st.y + 4 && x >= st.x && x < st.x + 4)
                        if (shape.row[y - st.y] & (1 << (x - st.x)))
                            is_active = true;

                    if (!is_active && y >= ghost_y && y < ghost_y + 4 && x >= st.x && x < st.x + 4)
                        if (shape.row[y - ghost_y] & (1 << (x - st.x)))
                            is_ghost = true;

                    if (is_active)
                        std::cout << colors[(int)st.piece + 1] << "  " << "\033[0m";
                    else if (is_ghost)
                        std::cout << "\033[90m[]\033[0m";
                    else if (st.board.rows[y] & (1ULL << x))
                        std::cout << "\033[47m  \033[0m";
                    else
                        std::cout << " .";
                }
                std::cout << "!>";

                if (y >= H - st.pending_garbage)
                    std::cout << "\033[31m*\033[0m";
                else
                    std::cout << " ";
            }
            move_cursor(offset_x, offset_y + 1 + H);
            std::cout << "==========================";
        }
    };
}

#endif // INCLUDE_TETRIS_TERMINAL_RENDERER_HPP
