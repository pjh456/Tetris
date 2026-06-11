#ifndef INCLUDE_TETRIS_SYSTEMS_INPUT_SYSTEM_HPP
#define INCLUDE_TETRIS_SYSTEMS_INPUT_SYSTEM_HPP

#include "tetris/input/input_mapper.hpp"

#ifdef _WIN32
#include <windows.h>
#include <conio.h>
#else
#include <termios.h>
#include <unistd.h>
#include <fcntl.h>
#include <cstdio>
#endif

namespace tetris::systems
{
    class InputSystem
    {
    public:
        InputSystem(tetris::input::InputMapper& mapper)
            : m_mapper(mapper)
        {
#ifdef _WIN32
            memset(m_prev_keys, 0, sizeof(m_prev_keys));
#endif
        }

        void initialize()
        {
            m_quit_requested = false;
            m_cycle_observed = false;
        }

        void shutdown() {}

        bool poll_action(tetris::core::Action& out_action)
        {
            using tetris::core::Action;

            m_cycle_observed = false;

#ifdef _WIN32
            struct KeyMapping { int vk; Action action; };
            static const KeyMapping arrow_keys[] = {
                {VK_LEFT,  Action::MoveLeft},
                {VK_RIGHT, Action::MoveRight},
                {VK_DOWN,  Action::SoftDrop},
                {VK_UP,    Action::RotateCW},
                {VK_SPACE, Action::HardDrop},
            };

            for (auto& km : arrow_keys)
            {
                SHORT state = GetAsyncKeyState(km.vk);
                bool pressed_now = (state & 0x8000) != 0;
                bool pressed_before = (m_prev_keys[km.vk] & 0x8000) != 0;
                m_prev_keys[km.vk] = state;

                if (pressed_now && !pressed_before)
                {
                    out_action = km.action;
                    return true;
                }
            }

            if (_kbhit())
            {
                int ch = _getch();
                if (ch == 0 || ch == 224) { _getch(); return false; }
                if (ch == 'q' || ch == 'Q') { m_quit_requested = true; return false; }
                if (ch == '\t' || ch == 'f' || ch == 'F') { m_cycle_observed = true; return false; }
                return m_mapper.resolve(ch, out_action);
            }
#else
            struct termios oldt, newt;
            int oldf;
            tcgetattr(STDIN_FILENO, &oldt);
            newt = oldt;
            newt.c_lflag &= ~(ICANON | ECHO);
            tcsetattr(STDIN_FILENO, TCSANOW, &newt);
            oldf = fcntl(STDIN_FILENO, F_GETFL, 0);
            fcntl(STDIN_FILENO, F_SETFL, oldf | O_NONBLOCK);

            int ch = getchar();
            if (ch != EOF)
            {
                if (ch == 27)
                {
                    if (getchar() == '[')
                    {
                        int arrow = getchar();
                        tcsetattr(STDIN_FILENO, TCSANOW, &oldt);
                        fcntl(STDIN_FILENO, F_SETFL, oldf);
                        switch (arrow)
                        {
                        case 'A': out_action = Action::RotateCW;  return true;
                        case 'B': out_action = Action::SoftDrop;  return true;
                        case 'C': out_action = Action::MoveRight; return true;
                        case 'D': out_action = Action::MoveLeft;  return true;
                        default: return false;
                        }
                    }
                    tcsetattr(STDIN_FILENO, TCSANOW, &oldt);
                    fcntl(STDIN_FILENO, F_SETFL, oldf);
                    return false;
                }
                if (ch == 'q' || ch == 'Q') { m_quit_requested = true; tcsetattr(STDIN_FILENO, TCSANOW, &oldt); fcntl(STDIN_FILENO, F_SETFL, oldf); return false; }
                if (ch == '\t' || ch == 'f' || ch == 'F') { m_cycle_observed = true; tcsetattr(STDIN_FILENO, TCSANOW, &oldt); fcntl(STDIN_FILENO, F_SETFL, oldf); return false; }
                bool resolved = m_mapper.resolve(ch, out_action);
                tcsetattr(STDIN_FILENO, TCSANOW, &oldt);
                fcntl(STDIN_FILENO, F_SETFL, oldf);
                return resolved;
            }
            tcsetattr(STDIN_FILENO, TCSANOW, &oldt);
            fcntl(STDIN_FILENO, F_SETFL, oldf);
#endif
            return false;
        }

        bool is_quit_requested() const { return m_quit_requested; }
        bool is_cycle_observed() const { return m_cycle_observed; }

    private:
        tetris::input::InputMapper& m_mapper;
#ifdef _WIN32
        SHORT m_prev_keys[256]{};
#endif
        bool m_quit_requested = false;
        bool m_cycle_observed = false;
    };
}

#endif
