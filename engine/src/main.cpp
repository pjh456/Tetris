#include <iostream>
#include <thread>
#include <chrono>
#include <vector>
#include <string>
#include <cstring>

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

// --- 局域网广播搜房助手类 (直接利用 ENet 的底层 Socket 实现跨平台) ---
class LANDraw
{
public:
    static constexpr uint16_t DISCOVERY_PORT = 7776;
    static constexpr const char *PING_MSG = "TETRIS_PING";
    static constexpr const char *PONG_MSG = "TETRIS_PONG";

    // 房主：在后台线程监听广播，并回发 PONG
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
                // 收到搜房请求，回发 PONG
                ENetBuffer reply;
                reply.data = (void *)PONG_MSG;
                reply.dataLength = strlen(PONG_MSG);
                enet_socket_send(sock, &sender, &reply, 1);
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(100));
        }
        enet_socket_destroy(sock);
    }

    // 客机：发送广播并收集回应的 IP
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
        enet_socket_send(sock, &bcast, &req, 1); // 发送 PING 广播

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

// --- TUI 渲染系统 ---
void move_cursor(int x, int y)
{
    std::cout << "\033[" << y << ";" << x << "H";
}

void clear_screen()
{
    std::cout << "\033[2J\033[H";
}

// 绘制单个玩家的盘面
void render_board(const State<10, 20> &st, int offset_x, int offset_y, const std::string &title)
{
    move_cursor(offset_x, offset_y);
    std::cout << "=== " << title << " ===";

    int ghost_y = get_ghost_y(st);
    const auto &shape = PIECES[(int)st.piece].rot[(int)st.rot];

    // ANSI 颜色：0=空, 1=I(青), 2=O(黄), 3=T(紫), 4=S(绿), 5=Z(红), 6=J(蓝), 7=L(白)
    const char *colors[] = {
        "\033[0m", "\033[46m", "\033[43m", "\033[45m",
        "\033[42m", "\033[41m", "\033[44m", "\033[47m"};

    for (int y = 0; y < 20; ++y)
    {
        move_cursor(offset_x, offset_y + 1 + y);
        std::cout << "<!"; // 左边框

        for (int x = 0; x < 10; ++x)
        {
            bool is_active = false;
            bool is_ghost = false;

            // 检查是否是当前正在控制的方块
            if (y >= st.y && y < st.y + 4 && x >= st.x && x < st.x + 4)
            {
                if (shape.row[y - st.y] & (1 << (x - st.x)))
                    is_active = true;
            }
            // 检查是否是 Ghost 方块 (投影)
            if (!is_active && y >= ghost_y && y < ghost_y + 4 && x >= st.x && x < st.x + 4)
            {
                if (shape.row[y - ghost_y] & (1 << (x - st.x)))
                    is_ghost = true;
            }

            if (is_active)
            {
                std::cout << colors[(int)st.piece + 1] << "  " << "\033[0m";
            }
            else if (is_ghost)
            {
                std::cout << "\033[90m[]\033[0m"; // 灰色中括号表示投影
            }
            else if (st.board.rows[y] & (1ULL << x))
            {
                std::cout << "\033[47m  \033[0m"; // 已锁定的方块用白色
            }
            else
            {
                std::cout << " ."; // 空地
            }
        }
        std::cout << "!>"; // 右边框
    }
    move_cursor(offset_x, offset_y + 21);
    std::cout << "==========================";
}

// --- 游戏主程序 ---
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

    // 扩大标准输出的缓冲区大小到 64KB，防止高帧率下 ANSI 序列被截断
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

        // 开启后台 UDP 答复线程
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

    // --- 游戏状态初始化 ---
    Engine<10, 20> local_engine;
    Engine<10, 20> remote_engine;
    bool game_started = false;

    // 网络事件绑定
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
            remote_engine.handle_action(pkt->action);
        }
        else if (header->type == PacketType::StateSync)
        {
            auto *pkt = reinterpret_cast<const PktStateSync<10, 20> *>(data);
            // 兜底同步：直接覆盖远端状态
            std::memcpy(remote_engine.state.board.rows, pkt->board_rows, sizeof(pkt->board_rows));
            remote_engine.state.piece = pkt->piece;
            remote_engine.state.rot = pkt->rot;
            remote_engine.state.x = pkt->x;
            remote_engine.state.y = pkt->y;
        }
    };

    net.on_disconnected = [&]()
    {
        game_started = false;
        clear_screen();
        std::cout << "\nPlayer disconnected. Game Over.\n";
        exit(0);
    };

    // --- 主循环 ---
    auto last_tick = std::chrono::steady_clock::now();
    auto last_sync = std::chrono::steady_clock::now();

    while (true)
    {
        net.tick(); // 维持 ENet 心跳与事件收发

        if (!game_started)
        {
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
            continue;
        }

        auto now = std::chrono::steady_clock::now();

        // 1. 处理本地输入
        if (_kbhit())
        {
            char c = _getch();
            Action act;
            bool valid_action = true;
            switch (c)
            {
            case 'a':
            case 'A':
                act = Action::MoveLeft;
                break;
            case 'd':
            case 'D':
                act = Action::MoveRight;
                break;
            case 's':
            case 'S':
                act = Action::SoftDrop;
                break;
            case 'w':
            case 'W':
            case ' ':
                act = Action::HardDrop;
                break;
            case 'j':
            case 'J':
                act = Action::RotateCCW;
                break;
            case 'k':
            case 'K':
                act = Action::RotateCW;
                break;
            case 'l':
            case 'L':
                act = Action::Hold;
                break;
            case 'q':
                exit(0);
            default:
                valid_action = false;
                break;
            }

            if (valid_action && !local_engine.game_over)
            {
                local_engine.handle_action(act);

                // 立即将操作发送给对手 (Channel 1, 保证顺序)
                PktPlayerAction action_pkt;
                action_pkt.header = {PacketType::PlayerAction, (u8)net.get_role()};
                action_pkt.action = act;
                net.send_packet(action_pkt, 1, true);
            }
        }

        // 2. 游戏自然下落 (重力 Tick)
        if (now - last_tick > std::chrono::milliseconds(500))
        {
            local_engine.tick();
            remote_engine.tick(); // 本地傀儡也必须下落，保证视觉同步
            last_tick = now;
        }

        // 3. 状态快照兜底同步 (每秒发 5 次 StateSync 包给对面)
        if (now - last_sync > std::chrono::milliseconds(200))
        {
            PktStateSync<10, 20> sync_pkt;
            sync_pkt.header = {PacketType::StateSync, (u8)net.get_role()};
            std::memcpy(sync_pkt.board_rows, local_engine.state.board.rows, sizeof(sync_pkt.board_rows));
            sync_pkt.piece = local_engine.state.piece;
            sync_pkt.rot = local_engine.state.rot;
            sync_pkt.x = local_engine.state.x;
            sync_pkt.y = local_engine.state.y;
            // Channel 2, 不可靠传输，丢弃也没事，保证最新即可
            net.send_packet(sync_pkt, 2, false);
            last_sync = now;
        }

        // 4. 渲染画面
        render_board(local_engine.state, 5, 2, "YOU (Local)");
        render_board(remote_engine.state, 40, 2, "OPPONENT (Remote)");

        move_cursor(5, 25);
        std::cout << "Controls: A/D(Move) S(Soft) W/Space(Hard) J/K(Rotate) L(Hold) Q(Quit)";

        // 这一帧的所有内容拼接完毕后，一次性发送给终端，避免画面撕裂和错乱
        std::cout << std::flush;

        std::this_thread::sleep_for(std::chrono::milliseconds(16)); // ~60FPS
    }

    discovery_running = false;
    if (discovery_thread.joinable())
        discovery_thread.join();
    return 0;
}