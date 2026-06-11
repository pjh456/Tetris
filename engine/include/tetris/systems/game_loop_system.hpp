#ifndef INCLUDE_TETRIS_SYSTEMS_GAME_LOOP_SYSTEM_HPP
#define INCLUDE_TETRIS_SYSTEMS_GAME_LOOP_SYSTEM_HPP

#include <vector>
#include <chrono>

#include "tetris/core/session.hpp"
#include "tetris/net/network_manager.hpp"

namespace tetris::systems
{
    template <tetris::core::u8 W, tetris::core::u8 H>
    class GameLoopSystem
    {
    public:
        GameLoopSystem(
            std::vector<tetris::core::GameSession<W, H>>& host_sessions,
            std::vector<bool>& player_seen,
            tetris::net::NetworkManager& net)
            : m_host_sessions(host_sessions), m_player_seen(player_seen), m_net(net)
        {
        }

        void update(bool game_started)
        {
            if (!game_started) return;

            auto now = std::chrono::steady_clock::now();
            if (now - m_last_tick < std::chrono::milliseconds(TICK_INTERVAL_MS)) return;
            m_last_tick = now;

            if (m_net.get_role() == tetris::net::NetworkManager::Role::Host)
            {
                for (size_t i = 0; i < m_host_sessions.size(); ++i)
                {
                    if (!m_player_seen[i]) continue;
                    m_host_sessions[i].tick();
                }
            }
        }

        void reset_timer()
        {
            m_last_tick = std::chrono::steady_clock::now();
        }

    private:
        std::vector<tetris::core::GameSession<W, H>>& m_host_sessions;
        std::vector<bool>& m_player_seen;
        tetris::net::NetworkManager& m_net;
        std::chrono::steady_clock::time_point m_last_tick{};
        static constexpr int TICK_INTERVAL_MS = 500;
    };
}

#endif
