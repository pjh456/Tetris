#include <iostream>
#include <thread>
#include <chrono>
#include <vector>
#include <string>
#include <cstring>
#include <algorithm>

#include <enet/enet.h>

#include "core/engine.hpp"
#include "network/network_manager.hpp"
#include "network/protocol.hpp"

// --- 跨平台非阻塞键盘输入 (kbhit / getch) ---
#ifdef _WIN32
#include <conio.h>
#else
#include <termios.h>
#include <unistd.h>
#include <fcntl.h>
int _kbhit()
{
    struct termios oldt, newt;
    int ch, oldf;
    tcgetattr(STDIN_FILENO, &oldt);
    newt = oldt;
    newt.c_lflag &= ~(ICANON | ECHO);
    tcsetattr(STDIN_FILENO, TCSANOW, &newt);
    oldf = fcntl(STDIN_FILENO, F_GETFL, 0);
    fcntl(STDIN_FILENO, F_SETFL, oldf | O_NONBLOCK);
    ch = getchar();
    tcsetattr(STDIN_FILENO, TCSANOW, &oldt);
    fcntl(STDIN_FILENO, F_SETFL, oldf);
    if (ch != EOF)
    {
        ungetc(ch, stdin);
        return 1;
    }
    return 0;
}
int _getch()
{
    struct termios oldt, newt;
    int ch;
    tcgetattr(STDIN_FILENO, &oldt);
    newt = oldt;
    newt.c_lflag &= ~(ICANON | ECHO);
    tcsetattr(STDIN_FILENO, TCSANOW, &newt);
    ch = getchar();
    tcsetattr(STDIN_FILENO, TCSANOW, &oldt);
    return ch;
}
#endif

using namespace tetris;
using namespace tetris::net;

class LANDraw
{
public:
    static constexpr uint16_t DISCOVERY_PORT = 7776;
    static constexpr const char *PING_MSG = "TETRIS_PING";
    static constexpr const char *PONG_MSG = "TETRIS_PONG";

    static void host_discovery_loop(bool &running)
    {
        ENetSocket sock = enet_socket_create(ENET_SOCKET_TYPE_DATAGRAM);
        ENetAddress addr;
        addr.host = ENET_HOST_ANY;
        addr.port = DISCOVERY_PORT;
        enet_socket_bind(sock, &addr);
        enet_socket_set_option(sock, ENET_SOCKOPT_NONBLOCK, 1);

        ENetBuffer buf;
        char data[64];
        buf.data = data;
        buf.dataLength = sizeof(data);

        while (running)
        {
            ENetAddress sender;
            int len = enet_socket_receive(sock, &sender, &buf, 1);
            if (len > 0 && strncmp(data, PING_MSG, strlen(PING_MSG)) == 0)
            {
                ENetBuffer reply;
                reply.data = (void *)PONG_MSG;
                reply.dataLength = strlen(PONG_MSG);
                enet_socket_send(sock, &sender, &reply, 1);
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(100));
        }
        enet_socket_destroy(sock);
    }

    static std::vector<std::string> scan_for_hosts()
    {
        std::vector<std::string> hosts;
        ENetSocket sock = enet_socket_create(ENET_SOCKET_TYPE_DATAGRAM);
        enet_socket_set_option(sock, ENET_SOCKOPT_BROADCAST, 1);
        enet_socket_set_option(sock, ENET_SOCKOPT_NONBLOCK, 1);

        ENetAddress bcast;
        bcast.host = ENET_HOST_BROADCAST;
        bcast.port = DISCOVERY_PORT;

        ENetBuffer req;
        req.data = (void *)PING_MSG;
        req.dataLength = strlen(PING_MSG);
        enet_socket_send(sock, &bcast, &req, 1);

        std::cout << "Scanning LAN for Tetris hosts for 2 seconds...\n";

        auto start = std::chrono::steady_clock::now();
        while (std::chrono::steady_clock::now() - start < std::chrono::seconds(2))
        {
            ENetAddress sender;
            ENetBuffer reply;
            char data[64];
            reply.data = data;
            reply.dataLength = sizeof(data);

            int len = enet_socket_receive(sock, &sender, &reply, 1);
            if (len > 0 && strncmp(data, PONG_MSG, strlen(PONG_MSG)) == 0)
            {
                char ip[64];
                enet_address_get_host_ip(&sender, ip, sizeof(ip));
                std::string ip_str(ip);
                if (std::find(hosts.begin(), hosts.end(), ip_str) == hosts.end())
                {
                    hosts.push_back(ip_str);
                    std::cout << "Found Host at: " << ip_str << std::endl;
                }
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(50));
        }
        enet_socket_destroy(sock);
        return hosts;
    }
};

void move_cursor(int x, int y) { std::cout << "\033[" << y << ";" << x << "H"; }
void clear_screen() { std::cout << "\033[2J\033[H"; }

void render_board(const State<10, 20> &st, int offset_x, int offset_y, const std::string &title)
{
    move_cursor(offset_x, offset_y);
    std::cout << "=== " << title << " ===";

    int ghost_y = get_ghost_y(st);
    const auto &shape = PIECES[(int)st.piece].rot[(int)st.rot];

    const char *colors[] = {
        "\033[0m", "\033[46m", "\033[43m", "\033[45m",
        "\033[42m", "\033[41m", "\033[44m", "\033[47m"};

    for (int y = 0; y < 20; ++y)
    {
        move_cursor(offset_x, offset_y + 1 + y);
        std::cout << "<!";

        for (int x = 0; x < 10; ++x)
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

        // 在右侧显示垃圾行警告槽
        if (y >= 20 - st.pending_garbage)
            std::cout << "\033[31m*\033[0m";
        else
            std::cout << " ";
    }
    move_cursor(offset_x, offset_y + 21);
    std::cout << "==========================";
}

int main()
{
#ifdef _WIN32
    HANDLE hOut = GetStdHandle(STD_OUTPUT_HANDLE);
    if (hOut != INVALID_HANDLE_VALUE)
    {
        DWORD mode = 0;
        GetConsoleMode(hOut, &mode);
        mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
        SetConsoleMode(hOut, mode);
    }
#endif

    static char stdout_buffer[65536];
    setvbuf(stdout, stdout_buffer, _IOFBF, sizeof(stdout_buffer));

    clear_screen();
    std::cout << "=== TETRIS LAN MULTIPLAYER ===\n";
    std::cout << "[0] Host Game (Create Room)\n";
    std::cout << "[1] Join Game (Search LAN)\n";
    std::cout << "Select (0/1): " << std::flush;

    int choice;
    std::cin >> choice;

    NetworkManager net;
    bool discovery_running = true;
    std::thread discovery_thread;

    if (choice == 0)
    {
        if (!net.start_server(7777))
        {
            std::cout << "Failed to start server.\n";
            return 1;
        }
        std::cout << "Waiting for a player to join...\n";
        discovery_thread = std::thread(LANDraw::host_discovery_loop, std::ref(discovery_running));
    }
    else
    {
        auto hosts = LANDraw::scan_for_hosts();
        if (hosts.empty())
        {
            std::cout << "No hosts found on LAN.\n";
            return 1;
        }
        std::cout << "Select host to join (0-" << hosts.size() - 1 << "): ";
        int host_idx;
        std::cin >> host_idx;
        if (host_idx < 0 || host_idx >= hosts.size())
            return 1;

        if (!net.connect_to_server(hosts[host_idx].c_str(), 7777))
        {
            std::cout << "Failed to connect.\n";
            return 1;
        }
    }

    Engine<10, 20> local_engine;
    Engine<10, 20> remote_engine;
    bool game_started = false;

    net.on_game_start = [&](uint32_t seed)
    {
        local_engine.reset(seed);
        remote_engine.reset(seed);
        game_started = true;
        clear_screen();
    };

    net.on_packet_received = [&](const uint8_t *data, size_t size)
    {
        auto *header = reinterpret_cast<const PacketHeader *>(data);
        if (header->type == PacketType::GameStart)
        {
            auto *pkt = reinterpret_cast<const PktGameStart *>(data);
            if (net.on_game_start)
                net.on_game_start(pkt->random_seed);
        }
        else if (header->type == PacketType::PlayerAction)
        {
            auto *pkt = reinterpret_cast<const PktPlayerAction *>(data);
            // 这里我们不需要处理 remote_engine 打出的垃圾行，避免双倍计算
            remote_engine.handle_action(pkt->action);
        }
        else if (header->type == PacketType::PlayerAttack)
        {
            // --- 联机对战核心逻辑：收到对手攻击包，挂载到垃圾行等待列 ---
            auto *pkt = reinterpret_cast<const PktPlayerAttack *>(data);
            local_engine.state.pending_garbage += pkt->lines;
        }
        else if (header->type == PacketType::StateSync)
        {
            auto *pkt = reinterpret_cast<const PktStateSync<10, 20> *>(data);
            std::memcpy(remote_engine.state.board.rows, pkt->board_rows, sizeof(pkt->board_rows));
            remote_engine.state.piece = pkt->piece;
            remote_engine.state.rot = pkt->rot;
            remote_engine.state.x = pkt->x;
            remote_engine.state.y = pkt->y;

            // 补充漏掉的对齐和状态字段
            remote_engine.state.hold = pkt->hold;
            remote_engine.state.hold_used = pkt->hold_used;
            remote_engine.state.pending_garbage = pkt->pending_garbage;
            remote_engine.state.rng = pkt->rng_state;
        }
    };

    net.on_disconnected = [&]()
    {
        game_started = false;
        clear_screen();
        std::cout << "\nPlayer disconnected. Game Over.\n";
        exit(0);
    };

    auto last_tick = std::chrono::steady_clock::now();
    auto last_sync = std::chrono::steady_clock::now();

    while (true)
    {
        net.tick();

        if (!game_started)
        {
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
            continue;
        }

        auto now = std::chrono::steady_clock::now();

        // 1. 处理本地输入 (处理上下左右被误解析的 Bug)
        if (_kbhit())
        {
            int c = _getch();
            Action act;
            bool valid_action = false;

#ifdef _WIN32
            // 拦截 Windows 方向键前缀 0 / 224
            if (c == 0 || c == 224)
            {
                int arrow = _getch();
                if (arrow == 72)
                {
                    act = Action::RotateCW;
                    valid_action = true;
                } // Up
                else if (arrow == 80)
                {
                    act = Action::SoftDrop;
                    valid_action = true;
                } // Down
                else if (arrow == 75)
                {
                    act = Action::MoveLeft;
                    valid_action = true;
                } // Left
                else if (arrow == 77)
                {
                    act = Action::MoveRight;
                    valid_action = true;
                } // Right
            }
#else
            // 拦截 Linux / macOS ANSI 转义序列 (\033[A ...)
            if (c == 27)
            {
                if (_kbhit() && _getch() == '[')
                {
                    if (_kbhit())
                    {
                        int arrow = _getch();
                        if (arrow == 'A')
                        {
                            act = Action::RotateCW;
                            valid_action = true;
                        } // Up
                        else if (arrow == 'B')
                        {
                            act = Action::SoftDrop;
                            valid_action = true;
                        } // Down
                        else if (arrow == 'D')
                        {
                            act = Action::MoveLeft;
                            valid_action = true;
                        } // Left
                        else if (arrow == 'C')
                        {
                            act = Action::MoveRight;
                            valid_action = true;
                        } // Right
                    }
                }
            }
#endif
            else
            {
                switch (c)
                {
                case 'a':
                case 'A':
                    act = Action::MoveLeft;
                    valid_action = true;
                    break;
                case 'd':
                case 'D':
                    act = Action::MoveRight;
                    valid_action = true;
                    break;
                case 's':
                case 'S':
                    act = Action::SoftDrop;
                    valid_action = true;
                    break;
                case ' ':
                    act = Action::HardDrop;
                    valid_action = true;
                    break;
                case 'j':
                case 'J':
                    act = Action::RotateCCW;
                    valid_action = true;
                    break;
                case 'w':
                case 'W':
                case 'k':
                case 'K':
                    act = Action::RotateCW;
                    valid_action = true;
                    break;
                case 'l':
                case 'L':
                case 'c':
                case 'C':
                    act = Action::Hold;
                    valid_action = true;
                    break;
                case 'q':
                case 'Q':
                    exit(0);
                }
            }

            if (valid_action && !local_engine.game_over)
            {
                auto res = local_engine.handle_action(act);

                // 发送操作包同步姿态
                PktPlayerAction action_pkt;
                action_pkt.header = {PacketType::PlayerAction, (u8)net.get_role()};
                action_pkt.action = act;
                net.send_packet(action_pkt, 1, true);

                // ！！关键修复：如果有伤害，发送给对手 ！！
                if (res.damage > 0)
                {
                    PktPlayerAttack atk_pkt;
                    atk_pkt.header = {PacketType::PlayerAttack, (u8)net.get_role()};
                    atk_pkt.lines = res.damage;
                    atk_pkt.hole_x = local_engine.state.rng % 10;
                    net.send_packet(atk_pkt, 1, true);
                }
            }
        }

        // 2. 游戏自然下落 (重力 Tick)
        if (now - last_tick > std::chrono::milliseconds(500))
        {
            auto res = local_engine.tick();
            remote_engine.tick();

            // 自然下落也可能锁定方块并造成伤害，同理发送
            if (res.damage > 0)
            {
                PktPlayerAttack atk_pkt;
                atk_pkt.header = {PacketType::PlayerAttack, (u8)net.get_role()};
                atk_pkt.lines = res.damage;
                atk_pkt.hole_x = local_engine.state.rng % 10;
                net.send_packet(atk_pkt, 1, true);
            }

            last_tick = now;
        }

        // 3. 状态快照兜底同步 (完善了遗漏的数据结构传输)
        if (now - last_sync > std::chrono::milliseconds(200))
        {
            PktStateSync<10, 20> sync_pkt;
            sync_pkt.header = {PacketType::StateSync, (u8)net.get_role()};
            std::memcpy(sync_pkt.board_rows, local_engine.state.board.rows, sizeof(sync_pkt.board_rows));
            sync_pkt.piece = local_engine.state.piece;
            sync_pkt.rot = local_engine.state.rot;
            sync_pkt.x = local_engine.state.x;
            sync_pkt.y = local_engine.state.y;

            // 加上核心字段
            sync_pkt.hold = local_engine.state.hold;
            sync_pkt.hold_used = local_engine.state.hold_used;
            sync_pkt.pending_garbage = local_engine.state.pending_garbage;
            sync_pkt.rng_state = local_engine.state.rng;

            net.send_packet(sync_pkt, 2, false);
            last_sync = now;
        }

        // 4. 渲染画面
        render_board(local_engine.state, 5, 2, "YOU (Local)");
        render_board(remote_engine.state, 40, 2, "OPPONENT (Remote)");

        move_cursor(5, 25);
        std::cout << "Controls: \033[36mArrow Keys / WASD\033[0m(Move&Drop) "
                     "\033[36mJ/K/W\033[0m(Rotate) "
                     "\033[36mL/C\033[0m(Hold) "
                     "\033[36mQ\033[0m(Quit)";
        std::cout << std::flush;

        std::this_thread::sleep_for(std::chrono::milliseconds(16)); // ~60FPS
    }

    // 退出程序前的清理
    discovery_running = false;
    if (discovery_thread.joinable())
        discovery_thread.join();

    return 0;
}