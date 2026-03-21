#include <iostream>
#include <chrono>
#include <thread>
#include <string>

#include "core/types.hpp"
#include "core/config.hpp"
#include "core/engine.hpp"

using namespace tetris;

#ifdef __linux__
#include <termios.h>
#include <unistd.h>
#include <fcntl.h>
#endif

#ifdef _WIN32
#include <conio.h>
#include <windows.h>
#endif

// ====== 非阻塞输入 ======
#ifdef __linux__
void init_terminal()
{
    termios tt{};
    tcgetattr(STDIN_FILENO, &tt);
    tt.c_lflag &= ~(ICANON | ECHO);
    tcsetattr(STDIN_FILENO, TCSANOW, &tt);
    fcntl(STDIN_FILENO, F_SETFL, O_NONBLOCK);
}
#endif

bool read_key(char &c)
{
#ifdef __linux__
    return read(STDIN_FILENO, &c, 1) > 0;
#elif defined(_WIN32)
    if (_kbhit())
    {
        c = _getch();
        return true;
    }
    return false;
#else
    return false;
#endif
}

void reset_cursor()
{
#ifdef __linux__
    std::cout << "\033[1;1H"; // 移至行首，避免 system("clear") 导致的闪屏
#elif defined(_WIN32)
    SetConsoleCursorPosition(GetStdHandle(STD_OUTPUT_HANDLE), {0, 0});
#endif
}

// ====== 游戏参数 ======
constexpr int W = 10;
constexpr int H = 20;

// ====== 渲染 ======
void render(const Engine<W, H> &engine)
{
    reset_cursor();
    const auto &s = engine.state;

    auto p2c = [](Piece p)
    { return "IOTSZJL"[(int)p]; };
    std::cout << "Hold: " << (engine.has_hold ? std::string(1, p2c(s.hold)) : " ")
              << " | Next: " << p2c(s.next[0]) << " \n";

    for (int y = 0; y < H; y++)
    {
        std::cout << "|";
        for (int x = 0; x < W; x++)
        {
            bool filled = (s.board.rows[y] >> x) & 1;

            if (!engine.game_over)
            {
                const auto &shape = PIECES[(int)s.piece].rot[(int)s.rot];
                for (int i = 0; i < 4; i++)
                {
                    if (s.y + i != y)
                        continue;
                    for (int j = 0; j < 4; j++)
                    {
                        if ((shape.row[i] & (1 << j)) && (s.x + j == x))
                            filled = true;
                    }
                }
            }
            std::cout << (filled ? "[]" : " .");
        }
        std::cout << "|\n";
    }
    std::cout << "+--------------------+\n";
    if (engine.game_over)
        std::cout << "==== GAME OVER! ====\n";
}

// ====== 主程序 ======
int main()
{
#ifdef __linux__
    std::cout << "\033[2J"; // 初始化清屏一次
    init_terminal();
#endif

    Engine<W, H> engine;
    engine.reset(1337);

    auto last = std::chrono::steady_clock::now();

    while (true)
    {
        char c;
        while (read_key(c))
        {
            if (c == 'q')
                return 0;
            if (engine.game_over)
                continue;

            if (c == 'a')
                engine.handle_action(Action::MoveLeft);
            if (c == 'd')
                engine.handle_action(Action::MoveRight);
            if (c == 's')
                engine.handle_action(Action::SoftDrop);
            if (c == 'w')
                engine.handle_action(Action::RotateCW);
            if (c == ' ')
                engine.handle_action(Action::HardDrop);
            if (c == 'c')
                engine.handle_action(Action::Hold);
        }

        if (engine.game_over)
        {
            render(engine);
            std::this_thread::sleep_for(std::chrono::milliseconds(100));
            continue;
        }

        auto now = std::chrono::steady_clock::now();
        if (std::chrono::duration_cast<std::chrono::milliseconds>(now - last).count() > 500)
        {
            engine.tick();
            last = now;
        }

        render(engine);
        std::this_thread::sleep_for(std::chrono::milliseconds(16));
    }
    return 0;
}