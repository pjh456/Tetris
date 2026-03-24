#include <iostream>
#include <thread>
#include <chrono>
#include <vector>
#include <string>
#include <cstring>
#include <algorithm>

#include <enet/enet.h>

#include "core/session.hpp"
#include "core/input_mapper.hpp"
#include "network/network_manager.hpp"
#include "network/net_game_driver.hpp"
#include "ui/terminal_renderer.hpp"

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

#ifdef _WIN32
static constexpr int KEY_ARROW_PREFIX_1 = 0;
static constexpr int KEY_ARROW_PREFIX_2 = 224;
static constexpr int KEY_ARROW_UP = 72;
static constexpr int KEY_ARROW_DOWN = 80;
static constexpr int KEY_ARROW_LEFT = 75;
static constexpr int KEY_ARROW_RIGHT = 77;
#else
static constexpr int KEY_ESC = 27;
static constexpr int KEY_ANSI_PREFIX = '[';
static constexpr int KEY_ARROW_UP = 'A';
static constexpr int KEY_ARROW_DOWN = 'B';
static constexpr int KEY_ARROW_LEFT = 'D';
static constexpr int KEY_ARROW_RIGHT = 'C';
#endif

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

static tetris::ui::TerminalRenderer renderer;

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

    renderer.clear_screen();
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

    GameSession<10, 20> local_session;
    GameSession<10, 20> remote_session;
    InputMapper input_mapper;
    bool game_started = false;
    NetGameDriver<10, 20> net_driver(net, local_session, remote_session);

    input_mapper.bind('a', Action::MoveLeft);
    input_mapper.bind('A', Action::MoveLeft);
    input_mapper.bind('d', Action::MoveRight);
    input_mapper.bind('D', Action::MoveRight);
    input_mapper.bind('s', Action::SoftDrop);
    input_mapper.bind('S', Action::SoftDrop);
    input_mapper.bind(' ', Action::HardDrop);
    input_mapper.bind('j', Action::RotateCCW);
    input_mapper.bind('J', Action::RotateCCW);
    input_mapper.bind('w', Action::RotateCW);
    input_mapper.bind('W', Action::RotateCW);
    input_mapper.bind('k', Action::RotateCW);
    input_mapper.bind('K', Action::RotateCW);
    input_mapper.bind('l', Action::Hold);
    input_mapper.bind('L', Action::Hold);
    input_mapper.bind('c', Action::Hold);
    input_mapper.bind('C', Action::Hold);

    auto on_game_start = [&](uint32_t seed)
    {
        local_session.reset(seed);
        remote_session.reset(seed);
        game_started = true;
        renderer.clear_screen();
    };
    net.on_game_start = on_game_start;
    net_driver.set_on_game_start(on_game_start);

    net.on_packet_received = [&](const uint8_t *data, size_t size)
    {
        net_driver.handle_packet(data, size);
    };

    net.on_disconnected = [&]()
    {
        game_started = false;
        renderer.clear_screen();
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
            if (c == KEY_ARROW_PREFIX_1 || c == KEY_ARROW_PREFIX_2)
            {
                int arrow = _getch();
                if (arrow == KEY_ARROW_UP)
                {
                    act = Action::RotateCW;
                    valid_action = true;
                }
                else if (arrow == KEY_ARROW_DOWN)
                {
                    act = Action::SoftDrop;
                    valid_action = true;
                }
                else if (arrow == KEY_ARROW_LEFT)
                {
                    act = Action::MoveLeft;
                    valid_action = true;
                }
                else if (arrow == KEY_ARROW_RIGHT)
                {
                    act = Action::MoveRight;
                    valid_action = true;
                }
            }
#else
            // 拦截 Linux / macOS ANSI 转义序列 (\033[A ...)
            if (c == KEY_ESC)
            {
                if (_kbhit() && _getch() == KEY_ANSI_PREFIX)
                {
                    if (_kbhit())
                    {
                        int arrow = _getch();
                        if (arrow == KEY_ARROW_UP)
                        {
                            act = Action::RotateCW;
                            valid_action = true;
                        }
                        else if (arrow == KEY_ARROW_DOWN)
                        {
                            act = Action::SoftDrop;
                            valid_action = true;
                        }
                        else if (arrow == KEY_ARROW_LEFT)
                        {
                            act = Action::MoveLeft;
                            valid_action = true;
                        }
                        else if (arrow == KEY_ARROW_RIGHT)
                        {
                            act = Action::MoveRight;
                            valid_action = true;
                        }
                    }
                }
            }
#endif
            else if (c == 'q' || c == 'Q')
            {
                exit(0);
            }
            else
            {
                valid_action = input_mapper.resolve(c, act);
            }

            if (valid_action && !local_session.is_game_over())
            {
                auto res = local_session.handle_action(act);

                // 发送操作包同步姿态
                net_driver.send_action(act);

                // ！！关键修复：如果有伤害，发送给对手 ！！
                if (res.damage > 0)
                {
                    net_driver.send_attack(res.damage, local_session.state().rng % 10);
                }
            }
        }

        // 2. 游戏自然下落 (重力 Tick)
        if (now - last_tick > std::chrono::milliseconds(500))
        {
            auto res = local_session.tick();
            remote_session.tick();

            // 自然下落也可能锁定方块并造成伤害，同理发送
            if (res.damage > 0)
            {
                net_driver.send_attack(res.damage, local_session.state().rng % 10);
            }

            last_tick = now;
        }

        // 3. 状态快照兜底同步 (完善了遗漏的数据结构传输)
        if (now - last_sync > std::chrono::milliseconds(200))
        {
            net_driver.send_state_sync();
            last_sync = now;
        }

        // 4. 渲染画面
        renderer.render_board(local_session.state(), 5, 2, "YOU (Local)");
        renderer.render_board(remote_session.state(), 40, 2, "OPPONENT (Remote)");

        renderer.move_cursor(5, 25);
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
