#ifndef INCLUDE_TETRIS_SYSTEMS_RENDER_SYSTEM_HPP
#define INCLUDE_TETRIS_SYSTEMS_RENDER_SYSTEM_HPP

#include <vector>
#include <chrono>
#include <iostream>
#include <string>

#include "tetris/core/session.hpp"
#include "tetris/net/network_manager.hpp"
#include "tetris/ui/terminal_renderer.hpp"

namespace tetris::systems
{
    template <tetris::core::u8 W, tetris::core::u8 H>
    class RenderSystem
    {
    public:
        RenderSystem(
            tetris::ui::TerminalRenderer& renderer,
            std::vector<tetris::core::GameSession<W, H>>& host_sessions,
            std::vector<tetris::core::GameSession<W, H>>& remote_sessions,
            std::vector<bool>& player_seen,
            tetris::net::NetworkManager& net,
            tetris::core::u8& observed_player_id,
            bool& observed_initialized)
            : m_renderer(renderer), m_host_sessions(host_sessions),
              m_remote_sessions(remote_sessions), m_player_seen(player_seen),
              m_net(net), m_observed_id(observed_player_id),
              m_observed_init(observed_initialized)
        {
        }

        void update(bool game_started)
        {
            if (!game_started)
            {
                render_wait_screen();
                return;
            }

            auto now = std::chrono::steady_clock::now();
            if (now - m_last_render < std::chrono::milliseconds(RENDER_INTERVAL_MS))
                return;
            m_last_render = now;

            bool is_host = (m_net.get_role() == tetris::net::NetworkManager::Role::Host);

            if (is_host)
            {
                if (!m_host_sessions.empty())
                    m_renderer.render_board(m_host_sessions[0].state(), 1, 1, "YOU (Host)");

                int offset_x = 30;
                for (size_t i = 1; i < m_host_sessions.size(); ++i)
                {
                    if (!m_player_seen[i]) continue;
                    m_renderer.render_board(m_host_sessions[i].state(), offset_x, 1, "P" + std::to_string(i));
                    offset_x += 26;
                }
            }
            else
            {
                if (!m_observed_init)
                {
                    for (size_t i = 0; i < m_player_seen.size(); ++i)
                    {
                        if (m_player_seen[i] && i != (size_t)m_net.local_player_id())
                        {
                            m_observed_id = (tetris::core::u8)i;
                            m_observed_init = true;
                            break;
                        }
                    }
                }

                if (m_observed_init && m_observed_id < m_remote_sessions.size())
                {
                    m_renderer.render_board(m_remote_sessions[m_observed_id].state(), 30, 1, "Opponent");
                }
            }

            std::cout << "\na/d=Move s=Drop w/k=Rotate j=RCCW space=HardDrop l/c=Hold q=Quit tab=Switch\n";
        }

    private:
        tetris::ui::TerminalRenderer& m_renderer;
        std::vector<tetris::core::GameSession<W, H>>& m_host_sessions;
        std::vector<tetris::core::GameSession<W, H>>& m_remote_sessions;
        std::vector<bool>& m_player_seen;
        tetris::net::NetworkManager& m_net;
        tetris::core::u8& m_observed_id;
        bool& m_observed_init;

        std::chrono::steady_clock::time_point m_last_render{};
        std::chrono::steady_clock::time_point m_last_wait_render{};
        static constexpr int RENDER_INTERVAL_MS = 16;
        static constexpr int WAIT_RENDER_INTERVAL_MS = 300;

        void render_wait_screen()
        {
            auto now = std::chrono::steady_clock::now();
            if (now - m_last_wait_render < std::chrono::milliseconds(WAIT_RENDER_INTERVAL_MS))
                return;
            m_last_wait_render = now;

            m_renderer.clear_screen();
            bool is_host = (m_net.get_role() == tetris::net::NetworkManager::Role::Host);

            if (is_host)
            {
                std::cout << "=== TETRIS HOST ===\n";
                std::cout << "Connected players: " << m_net.connected_count() << "\n";
                std::cout << "Press G to start the game!\n";
            }
            else
            {
                std::cout << "=== TETRIS CLIENT ===\n";
                std::cout << "Connected to server.\n";
                std::cout << "Waiting for host to start game...\n";
            }
        }
    };
}

#endif
