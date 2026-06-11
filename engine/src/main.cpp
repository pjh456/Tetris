#include <iostream>
#include <thread>
#include <chrono>
#include <vector>
#include <string>
#include <cstring>
#include <algorithm>
#include <enet/enet.h>
#include "tetris/core/session.hpp"
#include "tetris/input/input_mapper.hpp"
#include "tetris/net/network_manager.hpp"
#include "tetris/net/net_game_driver.hpp"
#include "tetris/ui/terminal_renderer.hpp"
#include "tetris/systems/input_system.hpp"
#include "tetris/systems/game_loop_system.hpp"
#include "tetris/systems/network_sync_system.hpp"
#include "tetris/systems/render_system.hpp"

using namespace tetris::core;
using namespace tetris::net;
using namespace tetris::input;
using namespace tetris::systems;
using namespace tetris::ui;

static std::vector<std::string> scan_for_hosts() {
    std::vector<std::string> hosts;
    ENetSocket sock = enet_socket_create(ENET_SOCKET_TYPE_DATAGRAM);
    enet_socket_set_option(sock, ENET_SOCKOPT_BROADCAST, 1);
    enet_socket_set_option(sock, ENET_SOCKOPT_NONBLOCK, 1);
    ENetAddress bcast; bcast.host = ENET_HOST_BROADCAST; bcast.port = 7776;
    ENetBuffer req; const char* ping = "TETRIS_PING"; const char* pong = "TETRIS_PONG";
    req.data = (void*)ping; req.dataLength = strlen(ping);
    enet_socket_send(sock, &bcast, &req, 1);
    auto start = std::chrono::steady_clock::now();
    while (std::chrono::steady_clock::now() - start < std::chrono::seconds(2)) {
        ENetAddress sender; char data[64]; ENetBuffer reply; reply.data = data; reply.dataLength = sizeof(data);
        if (enet_socket_receive(sock, &sender, &reply, 1) > 0 && strncmp(data, pong, strlen(pong)) == 0) {
            char ip[64]; enet_address_get_host_ip(&sender, ip, sizeof(ip));
            if (std::find(hosts.begin(), hosts.end(), ip) == hosts.end()) hosts.push_back(ip);
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(50));
    }
    enet_socket_destroy(sock);
    return hosts;
}

int main() {
#ifdef _WIN32
    HANDLE hOut = GetStdHandle(STD_OUTPUT_HANDLE);
    if (hOut != INVALID_HANDLE_VALUE) { DWORD mode = 0; GetConsoleMode(hOut, &mode); mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING; SetConsoleMode(hOut, mode); }
#endif
    static char outbuf[65536]; setvbuf(stdout, outbuf, _IOFBF, sizeof(outbuf));
    TerminalRenderer renderer; renderer.clear_screen();
    std::cout << "=== TETRIS LAN MULTIPLAYER ===\n[0] Host\n[1] Join\nSelect: " << std::flush;
    int choice; std::cin >> choice;

    NetworkManager net;
    if (choice == 0) { if (!net.start_server(7777)) { std::cout << "Failed to start server.\n"; return 1; } }
    else {
        auto hosts = scan_for_hosts();
        if (hosts.empty()) { std::cout << "No hosts found.\n"; return 1; }
        std::cout << "Select host (0-" << hosts.size()-1 << "): ";
        int idx; std::cin >> idx;
        if (idx < 0 || idx >= (int)hosts.size()) return 1;
        if (!net.connect_to_server(hosts[idx].c_str(), 7777)) { std::cout << "Failed to connect.\n"; return 1; }
    }

    GameSession<10,20> local_session;
    std::vector<GameSession<10,20>> remote_sessions, host_sessions;
    std::vector<bool> remote_seen, host_seen;
    InputMapper mapper;
    bool game_started = false, running = true;
    NetGameDriver<10,20> net_driver(net, local_session, local_session);
    u8 observed_id = 0xFF; bool observed_init = false;

    if (net.get_role() == NetworkManager::Role::Host) {
        host_sessions.resize(net.max_players()); host_seen.assign(net.max_players(), false);
        if (!host_seen.empty()) host_seen[0] = true;
    } else {
        remote_sessions.resize(net.max_players()); remote_seen.assign(net.max_players(), false);
    }

    auto bind = [&](int k, Action a) { mapper.bind(k, a); };
    bind('a',Action::MoveLeft); bind('A',Action::MoveLeft); bind('d',Action::MoveRight); bind('D',Action::MoveRight);
    bind('s',Action::SoftDrop); bind('S',Action::SoftDrop); bind('w',Action::RotateCW); bind('W',Action::RotateCW);
    bind('k',Action::RotateCW); bind('K',Action::RotateCW); bind('j',Action::RotateCCW); bind('J',Action::RotateCCW);
    bind('l',Action::Hold); bind('L',Action::Hold); bind('c',Action::Hold); bind('C',Action::Hold);

    InputSystem input(mapper);
    GameLoopSystem<10,20> game_loop(host_sessions, host_seen, net);
    NetworkSyncSystem<10,20> net_sync(net, host_sessions, net_driver, game_started, running, mapper);
    RenderSystem<10,20> render(renderer, host_sessions, remote_sessions,
        net.get_role()==NetworkManager::Role::Host ? host_seen : remote_seen, net, observed_id, observed_init);

    input.initialize(); net_sync.initialize();
    if (net.get_role() == NetworkManager::Role::Host) net_sync.start_lan_discovery();

    while (running) {
        net_sync.update();
        if (game_started) { Action act; if (input.poll_action(act)) { if (net.get_role()==NetworkManager::Role::Host) host_sessions[0].handle_action(act); else { local_session.handle_action(act); net_driver.send_action(act); } } game_loop.update(game_started); }
        if (input.is_quit_requested()) running = false;
        render.update(game_started);
        std::this_thread::sleep_for(std::chrono::milliseconds(16));
    }

    net_sync.stop_lan_discovery();
    renderer.clear_screen(); std::cout << "Thanks for playing!\n";
    return 0;
}
