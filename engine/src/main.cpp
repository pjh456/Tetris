#include <iostream>
#include <chrono>
#include <thread>
#include <string>
#include <sstream>
#include <cstdlib>

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

// ====== 终端与非阻塞输入 ======
#ifdef _WIN32
void init_terminal()
{
    // 激活 Windows 终端的 ANSI 颜色支持
    HANDLE hOut = GetStdHandle(STD_OUTPUT_HANDLE);
    DWORD dwMode = 0;
    GetConsoleMode(hOut, &dwMode);
    dwMode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
    SetConsoleMode(hOut, dwMode);
    std::cout << "\033[?25l"; // 隐藏光标
}
#elif defined(__linux__)
void init_terminal()
{
    termios tt{};
    tcgetattr(STDIN_FILENO, &tt);
    tt.c_lflag &= ~(ICANON | ECHO);
    tcsetattr(STDIN_FILENO, TCSANOW, &tt);
    fcntl(STDIN_FILENO, F_SETFL, O_NONBLOCK);
    std::cout << "\033[?25l"; // 隐藏光标
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
#endif
}

// ====== 视觉样式与颜色映射 ======
const char *get_color(Piece p)
{
    switch (p)
    {
    case Piece::I:
        return "\033[96m"; // 青色
    case Piece::O:
        return "\033[93m"; // 黄色
    case Piece::T:
        return "\033[95m"; // 紫色
    case Piece::S:
        return "\033[92m"; // 绿色
    case Piece::Z:
        return "\033[91m"; // 红色
    case Piece::J:
        return "\033[94m"; // 蓝色
    case Piece::L:
        return "\033[38;5;208m"; // 橘色 (256色)
    }
    return "\033[0m";
}

// 渲染侧边栏的迷你方块
std::string render_mini(Piece p, int row, bool active)
{
    if (!active)
        return "        "; // 8个空格占位

    // 严格控制每行 8 字符宽度
    const char *shapes[7][2] = {
        {"[][][][]", "        "}, // I
        {"  [][]  ", "  [][]  "}, // O
        {"  []    ", "[][][]  "}, // T
        {"  [][]  ", "[][]    "}, // S
        {"[][]    ", "  [][]  "}, // Z
        {"[]      ", "[][][]  "}, // J
        {"    []  ", "[][][]  "}  // L (修复: 补齐为8字符，并修正形状)
    };

    return std::string(get_color(p)) + shapes[(int)p][row] + "\033[0m";
}

constexpr int W = 10;
constexpr int H = 20;

// ====== 无闪烁的 TUI 渲染 ======
void render(const Engine<W, H> &engine)
{
    std::ostringstream out;
    out << "\033[1;1H"; // 光标移至左上角，避免闪烁

    const auto &s = engine.state;
    int ghost_y = get_ghost_y(s);

    // 0: 空, 1: 锁定块, 2: 幽灵块, 3: 活动块
    int grid[H][W] = {0};

    // 1. 填入已锁定的死区方块
    for (int y = 0; y < H; y++)
    {
        for (int x = 0; x < W; x++)
        {
            if ((s.board.rows[y] >> x) & 1)
                grid[y][x] = 1;
        }
    }

    if (!engine.game_over)
    {
        const auto &shape = PIECES[(int)s.piece].rot[(int)s.rot];

        // 2. 填入 Ghost 块
        for (int i = 0; i < 4; i++)
        {
            for (int j = 0; j < 4; j++)
            {
                if (shape.row[i] & (1 << j))
                {
                    int xx = s.x + j, yy = ghost_y + i;
                    if (yy >= 0 && yy < H && xx >= 0 && xx < W)
                        grid[yy][xx] = 2;
                }
            }
        }

        // 3. 填入 Active 块 (会覆盖该位置的 Ghost 块)
        for (int i = 0; i < 4; i++)
        {
            for (int j = 0; j < 4; j++)
            {
                if (shape.row[i] & (1 << j))
                {
                    int xx = s.x + j, yy = s.y + i;
                    if (yy >= 0 && yy < H && xx >= 0 && xx < W)
                        grid[yy][xx] = 3;
                }
            }
        }
    }

    // --- 开始逐行绘制 ---
    out << "           ╔════════════════════╗\033[K\n";

    for (int y = 0; y < H; y++)
    {
        // [左侧面板: HOLD] (宽 11 字符)
        if (y == 1)
            out << "   HOLD    ";
        else if (y == 2)
            out << " " << render_mini(s.hold, 0, engine.has_hold) << "  ";
        else if (y == 3)
            out << " " << render_mini(s.hold, 1, engine.has_hold) << "  ";
        else
            out << "           ";

        // [棋盘区]
        out << "║";
        for (int x = 0; x < W; x++)
        {
            if (grid[y][x] == 0)
                out << " .";
            else if (grid[y][x] == 1)
                out << "\033[37m[]\033[0m"; // 锁定块：白色
            else if (grid[y][x] == 2)
                out << "\033[90m[]\033[0m"; // Ghost：深灰色
            else if (grid[y][x] == 3)
                out << get_color(s.piece) << "[]\033[0m"; // 当前块：高亮色
        }
        out << "║";

        // [右侧面板: NEXT]
        if (y == 1)
            out << "   NEXT";
        else if (y == 2)
            out << " " << render_mini(s.next[0], 0, true);
        else if (y == 3)
            out << " " << render_mini(s.next[0], 1, true);
        else if (y == 5)
            out << " " << render_mini(s.next[1], 0, true);
        else if (y == 6)
            out << " " << render_mini(s.next[1], 1, true);
        else if (y == 8)
            out << " " << render_mini(s.next[2], 0, true);
        else if (y == 9)
            out << " " << render_mini(s.next[2], 1, true);

        out << "\033[K\n";
    }

    out << "           ╚════════════════════╝\033[K\n";

    if (engine.game_over)
    {
        out << "              \033[91m=== GAME OVER ===\033[0m\033[K\n";
    }
    else
    {
        out << "            [a/d] Move  [w] Rotate\033[K\n"
            << "            [s] Drop    [SPACE] Hard Drop\033[K\n"
            << "            [c] Hold    [q] Quit\033[K\n";
    }

    // 只有一次 IO 操作，彻底杜绝闪屏
    std::cout << out.str() << std::flush;
}

// ====== 主程序 ======
int main()
{
    // 注册退出时恢复光标
    std::atexit(
        []()
        { std::cout << "\033[?25h\n"; });

    init_terminal();
    std::cout << "\033[2J"; // 清空整个屏幕

    Engine<W, H> engine;
    engine.reset(1337); // 固定种子或换成 time(0)

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
            std::this_thread::sleep_for(
                std::chrono::milliseconds(100));
            continue;
        }

        auto now = std::chrono::steady_clock::now();
        if (std::chrono::duration_cast<
                std::chrono::milliseconds>(now - last)
                .count() > 500)
        {
            engine.tick();
            last = now;
        }

        render(engine);
        std::this_thread::sleep_for(std::chrono::milliseconds(16));
    }
    return 0;
}