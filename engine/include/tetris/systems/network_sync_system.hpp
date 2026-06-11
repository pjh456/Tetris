#ifndef INCLUDE_TETRIS_SYSTEMS_NETWORK_SYNC_SYSTEM_HPP
#define INCLUDE_TETRIS_SYSTEMS_NETWORK_SYNC_SYSTEM_HPP

#include <vector>
#include <string>
#include <chrono>
#include <mutex>
#include <future>
#include <iostream>
#include <cstring>

#include <enet/enet.h>

#include "tetris/core/session.hpp"
#include "tetris/core/attack.hpp"
#include "tetris/net/network_manager.hpp"
#include "tetris/net/net_game_driver.hpp"
#include "tetris/input/input_mapper.hpp"

namespace tetris::systems
{
    template <tetris::core::u8 W, tetris::core::u8 H>
    class NetworkSyncSystem
    {
    public:
        NetworkSyncSystem(
            tetris::net::NetworkManager& net,
            std::vector<tetris::core::GameSession<W, H>>& host_sessions,
            tetris::net::NetGameDriver<W, H>& net_driver,
            bool& game_started,
            bool& running,
            tetris::input::InputMapper& mapper)
            : m_net(net), m_host_sessions(host_sessions), m_net_driver(net_driver),
              m_game_started(game_started), m_running(running), m_mapper(mapper)
        {
        }

        void initialize()
        {
            bool is_host = (m_net.get_role() == tetris::net::NetworkManager::Role::Host);

            m_net.on_game_start = [this](uint32_t seed) {
                if (m_net.get_role() == tetris::net::NetworkManager::Role::Host)
                {
                    for (size_t i = 0; i < m_host_sessions.size(); ++i)
                    {
                        if (m_host_sessions.size() > 0)
                            break;
                        m_host_sessions[i].reset(seed);
                    }
                    for (size_t i = 1; i < m_host_sessions.size(); ++i)
                        m_host_sessions[i].reset(seed);
                }
                m_game_started = true;
            };

            m_net.on_packet_received = [this](const uint8_t* data, size_t size) {
                auto* header = reinterpret_cast<const tetris::net::PacketHeader*>(data);

                if (m_net.get_role() == tetris::net::NetworkManager::Role::Host)
                {
                    uint8_t from_pid = header->player_id;
                    handle_host_packet(data, size, from_pid);
                }
                else
                {
                    handle_client_packet(data, size);
                }
            };

            m_net.on_disconnected = [this]() {
                m_game_started = false;
                std::cout << "\nPlayer disconnected. Game Over.\n";
            };
        }

        void update()
        {
            m_net.tick();

            if (m_game_started)
            {
                auto now = std::chrono::steady_clock::now();
                if (m_net.get_role() == tetris::net::NetworkManager::Role::Host)
                {
                    if (now - m_last_sync >= std::chrono::milliseconds(SYNC_INTERVAL_MS))
                    {
                        m_last_sync = now;
                    }
                }
            }

            if (m_net.get_role() == tetris::net::NetworkManager::Role::Host && m_discovery_running)
                poll_lan_discovery();
        }

        void start_lan_discovery()
        {
            if (m_discovery_running) return;

            m_discovery_sock = enet_socket_create(ENET_SOCKET_TYPE_DATAGRAM);
            if (m_discovery_sock == ENET_SOCKET_NULL) return;

            ENetAddress addr;
            addr.host = ENET_HOST_ANY;
            addr.port = DISCOVERY_PORT;

            if (enet_socket_bind(m_discovery_sock, &addr) < 0)
            {
                enet_socket_destroy(m_discovery_sock);
                m_discovery_sock = ENET_SOCKET_NULL;
                return;
            }

            enet_socket_set_option(m_discovery_sock, ENET_SOCKOPT_NONBLOCK, 1);
            m_discovery_running = true;
        }

        void stop_lan_discovery()
        {
            if (!m_discovery_running) return;
            m_discovery_running = false;
            if (m_discovery_sock != ENET_SOCKET_NULL)
            {
                enet_socket_destroy(m_discovery_sock);
                m_discovery_sock = ENET_SOCKET_NULL;
            }
        }

        void poll_lan_discovery()
        {
            if (m_discovery_sock == ENET_SOCKET_NULL) return;

            std::lock_guard<std::mutex> lock(m_socket_mutex);

            ENetBuffer buf;
            char data[64];
            buf.data = data;
            buf.dataLength = sizeof(data);

            ENetAddress sender;
            int len = enet_socket_receive(m_discovery_sock, &sender, &buf, 1);
            if (len > 0 && strncmp(data, PING_MSG, strlen(PING_MSG)) == 0)
            {
                ENetBuffer reply;
                reply.data = (void*)PONG_MSG;
                reply.dataLength = strlen(PONG_MSG);
                enet_socket_send(m_discovery_sock, &sender, &reply, 1);
            }
        }

        tetris::core::u8 find_opponent(tetris::core::u8 current_pid) const
        {
            return (current_pid == 0) ? (tetris::core::u8)1 : (tetris::core::u8)0;
        }

    private:
        tetris::net::NetworkManager& m_net;
        std::vector<tetris::core::GameSession<W, H>>& m_host_sessions;
        tetris::net::NetGameDriver<W, H>& m_net_driver;
        bool& m_game_started;
        bool& m_running;
        tetris::input::InputMapper& m_mapper;

        std::chrono::steady_clock::time_point m_last_sync{};
        static constexpr int SYNC_INTERVAL_MS = 200;

        ENetSocket m_discovery_sock = ENET_SOCKET_NULL;
        bool m_discovery_running = false;
        std::mutex m_socket_mutex;

        static constexpr uint16_t DISCOVERY_PORT = 7776;
        static constexpr const char* PING_MSG = "TETRIS_PING";
        static constexpr const char* PONG_MSG = "TETRIS_PONG";

        void handle_host_packet(const uint8_t* data, size_t size, tetris::core::u8 from_pid)
        {
            (void)size;
            auto* header = reinterpret_cast<const tetris::net::PacketHeader*>(data);

            if (header->type == tetris::net::PacketType::PlayerAction)
            {
                auto* pkt = reinterpret_cast<const tetris::net::PktPlayerAction*>(data);
                if (from_pid >= m_host_sessions.size()) return;

                tetris::core::AttackResult res = m_host_sessions[from_pid].handle_action(pkt->action);

                if (res.damage > 0)
                {
                    tetris::core::u8 target = find_opponent(from_pid);
                    if (target < m_host_sessions.size() && m_net.is_player_connected(target))
                    {
                        m_host_sessions[target].state().pending_garbage += (tetris::core::u8)res.damage;
                    }
                }
            }
        }

        void handle_client_packet(const uint8_t* data, size_t size)
        {
            m_net_driver.handle_packet(data, size);
        }
    };
}

#endif
