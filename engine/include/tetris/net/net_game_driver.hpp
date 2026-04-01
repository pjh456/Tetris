#ifndef INCLUDE_TETRIS_NET_GAME_DRIVER_HPP
#define INCLUDE_TETRIS_NET_GAME_DRIVER_HPP

#include <cstddef>
#include <functional>
#include <cstring>

#include "tetris/core/session.hpp"
#include "tetris/core/snapshot.hpp"
#include "tetris/net/network_manager.hpp"
#include "tetris/net/protocol.hpp"

namespace tetris::net
{
    using core::Action;
    using core::GameSession;
    using core::u8;

    template <u8 W, u8 H>
    class NetGameDriver
    {
    private:
        NetworkManager &m_net;
        GameSession<W, H> &m_local;
        GameSession<W, H> &m_remote;
        std::function<void(uint32_t seed)> m_on_game_start;

    public:
        NetGameDriver(
            NetworkManager &net,
            GameSession<W, H> &local,
            GameSession<W, H> &remote)
            : m_net(net), m_local(local), m_remote(remote)
        {
        }

        void set_on_game_start(std::function<void(uint32_t seed)> cb)
        {
            m_on_game_start = std::move(cb);
        }

        void handle_packet(const uint8_t *data, size_t size)
        {
            (void)size;
            auto *header = reinterpret_cast<const PacketHeader *>(data);
            if (header->type == PacketType::GameStart)
            {
                auto *pkt = reinterpret_cast<const PktGameStart *>(data);
                if (m_on_game_start)
                    m_on_game_start(pkt->random_seed);
            }
            else if (header->type == PacketType::PlayerAction)
            {
                auto *pkt = reinterpret_cast<const PktPlayerAction *>(data);
                // 这里不需要处理 remote_session 打出的垃圾行，避免双倍计算
                m_remote.handle_action(pkt->action);
            }
            else if (header->type == PacketType::PlayerAttack)
            {
                // 收到对手攻击包，挂载到垃圾行等待列
                auto *pkt = reinterpret_cast<const PktPlayerAttack *>(data);
                m_local.state().pending_garbage += pkt->lines;
            }
            else if (header->type == PacketType::StateSync)
            {
                auto *pkt = reinterpret_cast<const PktStateSync<W, H> *>(data);
                auto &rst = m_remote.state();
                std::memcpy(rst.board.rows, pkt->board_rows, sizeof(pkt->board_rows));
                rst.piece = pkt->piece;
                rst.rot = pkt->rot;
                rst.x = pkt->x;
                rst.y = pkt->y;

                rst.hold = pkt->hold;
                rst.hold_used = pkt->hold_used;
                rst.pending_garbage = pkt->pending_garbage;
                rst.rng = pkt->rng_state;
            }
        }

        void send_action(Action act)
        {
            PktPlayerAction action_pkt;
            action_pkt.header = {PacketType::PlayerAction, m_net.local_player_id()};
            action_pkt.action = act;
            if (m_net.get_role() == NetworkManager::Role::Host)
                m_net.broadcast_packet(action_pkt, 1, true);
            else
                m_net.send_packet(action_pkt, 1, true);
        }

        void send_attack(u8 lines, u8 hole_x)
        {
            PktPlayerAttack atk_pkt;
            atk_pkt.header = {PacketType::PlayerAttack, m_net.local_player_id()};
            atk_pkt.lines = lines;
            atk_pkt.hole_x = hole_x;
            if (m_net.get_role() == NetworkManager::Role::Host)
                m_net.broadcast_packet(atk_pkt, 1, true);
            else
                m_net.send_packet(atk_pkt, 1, true);
        }

        void send_state_sync()
        {
            PktStateSync<W, H> sync_pkt;
            sync_pkt.header = {PacketType::StateSync, m_net.local_player_id()};
            auto snap = make_snapshot(m_local.state());
            std::memcpy(sync_pkt.board_rows, snap.board_rows, sizeof(sync_pkt.board_rows));
            sync_pkt.piece = snap.piece;
            sync_pkt.rot = snap.rot;
            sync_pkt.x = snap.x;
            sync_pkt.y = snap.y;
            sync_pkt.hold = snap.hold;
            sync_pkt.hold_used = snap.hold_used;
            sync_pkt.pending_garbage = snap.pending_garbage;
            sync_pkt.rng_state = snap.rng;
            if (m_net.get_role() == NetworkManager::Role::Host)
                m_net.broadcast_packet(sync_pkt, 2, false);
            else
                m_net.send_packet(sync_pkt, 2, false);
        }
    };
}

#endif // INCLUDE_TETRIS_NET_GAME_DRIVER_HPP
