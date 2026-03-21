#include <iostream>
#include <chrono>
#include <thread>
#include <cstdlib>

#include "core/types.hpp"
#include "core/config.hpp"
#include "core/piece.hpp"
#include "core/srs.hpp"
#include "core/board.hpp"
#include "core/state.hpp"
#include "core/rules.hpp"

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

// ====== 非阻塞输入（Linux）======
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

void clear_screen()
{
#ifdef __linux__
    system("clear");
#elif defined(_WIN32)
    HANDLE h = GetStdHandle(STD_OUTPUT_HANDLE);
    CONSOLE_SCREEN_BUFFER_INFO csbi;
    GetConsoleScreenBufferInfo(h, &csbi);

    DWORD size = csbi.dwSize.X * csbi.dwSize.Y;
    DWORD written;
    FillConsoleOutputCharacter(h, ' ', size, {0, 0}, &written);
    SetConsoleCursorPosition(h, {0, 0});
#endif
}

// ====== 游戏参数 ======
constexpr int W = 10;
constexpr int H = 20;

using Game = State<W, H>;

// ====== 随机 piece ======
Piece rand_piece()
{
    return static_cast<Piece>(rand() % 7);
}

// ====== 渲染 ======
void render(const Game &s)
{
    clear_screen();

    for (int y = 0; y < H; y++)
    {
        std::cout << "|";

        for (int x = 0; x < W; x++)
        {
            bool filled = (s.board.rows[y] >> x) & 1;

            // 当前方块
            const auto &shape =
                PIECES[(int)s.piece].rot[(int)s.rot];

            for (int i = 0; i < 4; i++)
            {
                int yy = s.y + i;
                if (yy != y)
                    continue;

                for (int j = 0; j < 4; j++)
                {
                    if (shape.row[i] & (1 << j))
                    {
                        int xx = s.x + j;
                        if (xx == x)
                            filled = true;
                    }
                }
            }

            std::cout << (filled ? "#" : " ");
        }

        std::cout << "|\n";
    }

    std::cout << "+----------+\n";
}

// ====== 初始化 ======
void spawn(Game &s)
{
    s.piece = rand_piece();
    s.rot = Rot::R0;
    s.x = 3;
    s.y = 0;
}

// ====== 主程序 ======
int main()
{
#ifdef __linux__
    init_terminal();
#endif

    Game game{};
    spawn(game);

    auto last = std::chrono::steady_clock::now();

    while (true)
    {
        // ===== 输入 =====
        char c;
        while (read_key(c))
        {
            if (c == 'a')
                try_move(game, -1, 0);
            if (c == 'd')
                try_move(game, 1, 0);
            if (c == 's')
                try_move(game, 0, 1);
            if (c == 'w')
                try_rotate(
                    game,
                    static_cast<Rot>(
                        ((int)game.rot + 1) & 3));
            if (c == ' ')
                hard_drop(game);
        }

        // ===== 重力 =====
        auto now = std::chrono::steady_clock::now();
        if (std::chrono::duration_cast<
                std::chrono::milliseconds>(now - last)
                .count() > 500)
        {
            if (can_place(game, game.x, game.y + 1, game.rot))
            {
                game.y++;
            }
            else
            {
                lock_piece(game);
                spawn(game);

                if (!can_place(game, game.x, game.y, game.rot))
                {
                    std::cout << "Game Over\n";
                    break;
                }
            }

            last = now;
        }

        // ===== 渲染 =====
        render(game);

        std::this_thread::sleep_for(std::chrono::milliseconds(16));
    }

    return 0;
}