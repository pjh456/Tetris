// --- START OF FILE network_manager.hpp ---
#ifndef INCLUDE_TETRIS_NETWORK_MANAGER_HPP
#define INCLUDE_TETRIS_NETWORK_MANAGER_HPP

#include <enet/enet.h>
#include <algorithm>
#include <functional>
#include <iostream>
#include <unordered_map>
#include <vector>

#include "protocol.hpp"

namespace tetris::net
{
    class NetworkManager
    {
    public:
        enum class Role
        {
            None,
            Host,
            Client
        };

        // --- 事件回调函数 (供外部游戏循环挂载) ---
        // 当双方建立连接并准备好开始时触发，下发相同的种子
        std::function<void(uint32_t seed)> on_game_start;
        // 收到对手的数据包时触发
        std::function<void(const uint8_t *data, size_t size)> on_packet_received;
        // 断开连接时触发
        std::function<void()> on_disconnected;

    private:
        ENetHost *host = nullptr;
        ENetPeer *peer = nullptr; // 客机只需要记录服务器 peer
        std::vector<ENetPeer *> peers_; // 服务器端保存所有客户端 peer
        std::unordered_map<ENetPeer *, u8> peer_ids_;
        Role role = Role::None;
        u8 local_player_id_ = 0;
        u8 max_players_ = 8;
        u8 next_player_id_ = 1;

    public:
        NetworkManager()
        {
            if (enet_initialize() != 0)
            {
                std::cerr << "Failed to initialize ENet!" << std::endl;
            }
        }

        ~NetworkManager()
        {
            disconnect();
            if (host)
                enet_host_destroy(host);
            enet_deinitialize();
        }

        // --- 作为房主启动 (监听端口) ---
        bool start_server(uint16_t port, u8 max_players = 8)
        {
            ENetAddress address;
            address.host = ENET_HOST_ANY;
            address.port = port;

            // 参数: 地址, 最大连接数(1v1所以是1), 通道数(2), 入带宽(0=不限), 出带宽(0=不限)
            host = enet_host_create(&address, max_players, 3, 0, 0);
            if (!host)
                return false;

            role = Role::Host;
            local_player_id_ = 0;
            max_players_ = max_players;
            next_player_id_ = 1;
            std::cout << "Server started on port " << port << std::endl;
            return true;
        }

        // --- 作为客机连接 (连接到指定IP) ---
        bool connect_to_server(const char *ip, uint16_t port)
        {
            // 客机不需要绑定特定端口，传 NULL
            host = enet_host_create(NULL, 1, 3, 0, 0);
            if (!host)
                return false;

            ENetAddress address;
            enet_address_set_host(&address, ip);
            address.port = port;

            // 发起连接，通道数设为 2
            peer = enet_host_connect(host, &address, 2, 0);
            if (!peer)
                return false;

            role = Role::Client;
            local_player_id_ = 0;
            std::cout << "Connecting to " << ip << ":" << port << "..." << std::endl;
            return true;
        }

        void disconnect()
        {
            if (peer)
            {
                enet_peer_disconnect(peer, 0);
                // 强制发送断开包
                enet_host_flush(host);
                peer = nullptr;
            }
            peers_.clear();
            peer_ids_.clear();
            role = Role::None;
        }

        // --- 发送任意数据包 (模板包装，方便使用) ---
        template <typename T>
        void send_packet(const T &packet_data, uint8_t channel = 0, bool reliable = true)
        {
            if (!peer)
                return;

            u32 flags = reliable ? ENET_PACKET_FLAG_RELIABLE : 0;
            ENetPacket *packet = enet_packet_create(&packet_data, sizeof(T), flags);

            enet_peer_send(peer, channel, packet);
        }

        template <typename T>
        void send_packet_to(ENetPeer *to_peer, const T &packet_data, uint8_t channel = 0, bool reliable = true)
        {
            if (!to_peer)
                return;

            u32 flags = reliable ? ENET_PACKET_FLAG_RELIABLE : 0;
            ENetPacket *packet = enet_packet_create(&packet_data, sizeof(T), flags);

            enet_peer_send(to_peer, channel, packet);
        }

        // --- 核心网络循环 (每帧调用) ---
        void tick()
        {
            if (!host)
                return;

            ENetEvent event;
            // 非阻塞轮询网络事件 (timeout = 0)
            while (enet_host_service(host, &event, 0) > 0)
            {
                switch (event.type)
                {
                case ENET_EVENT_TYPE_CONNECT:
                    peer = event.peer;
                    std::cout << "Connected to a peer!" << std::endl;

                    // 【极简对战逻辑】：如果是 Server，有人连进来就直接发 GameStart 并下发随机数种子！
                    if (role == Role::Host)
                    {
                        if (peers_.size() >= max_players_)
                        {
                            enet_peer_disconnect(event.peer, 0);
                            break;
                        }
                        peers_.push_back(event.peer);

                        PktGameStart start_pkt;
                        start_pkt.header = {PacketType::GameStart, 0};
                        start_pkt.random_seed = 202405; // MVP 先写死或者用 time()

                        send_packet(start_pkt, 0, true);

                        // Server 本地也触发 GameStart
                        if (on_game_start)
                            on_game_start(start_pkt.random_seed);
                    }
                    else if (role == Role::Client)
                    {
                        PktClientJoin join_pkt;
                        join_pkt.header = {PacketType::ClientJoin, 0};
                        send_packet(join_pkt, 0, true);
                    }
                    break;

                case ENET_EVENT_TYPE_RECEIVE:
                    // 将收到的数据抛给外层游戏逻辑处理
                    if (event.packet->dataLength >= sizeof(PacketHeader))
                    {
                        auto *header = reinterpret_cast<const PacketHeader *>(event.packet->data);
                        if (header->type == PacketType::ClientJoin && role == Role::Host)
                        {
                            if (peer_ids_.count(event.peer) == 0 && next_player_id_ < max_players_)
                            {
                                u8 assigned = next_player_id_++;
                                peer_ids_[event.peer] = assigned;

                                PktServerAccept accept_pkt;
                                accept_pkt.header = {PacketType::ServerAccept, 0};
                                accept_pkt.assigned_player_id = assigned;
                                accept_pkt.max_players = max_players_;
                                send_packet_to(event.peer, accept_pkt, 0, true);
                            }
                        }
                        else if (header->type == PacketType::ServerAccept && role == Role::Client)
                        {
                            if (event.packet->dataLength >= sizeof(PktServerAccept))
                            {
                                auto *pkt = reinterpret_cast<const PktServerAccept *>(event.packet->data);
                                local_player_id_ = pkt->assigned_player_id;
                                max_players_ = pkt->max_players;
                            }
                        }
                        else if (on_packet_received)
                        {
                            on_packet_received(event.packet->data, event.packet->dataLength);
                        }
                    }
                    // ENet 要求手动销毁收到的 packet
                    enet_packet_destroy(event.packet);
                    break;

                case ENET_EVENT_TYPE_DISCONNECT:
                    std::cout << "Peer disconnected." << std::endl;
                    peer = nullptr;
                    if (role == Role::Host)
                    {
                        peer_ids_.erase(event.peer);
                        peers_.erase(
                            std::remove(peers_.begin(), peers_.end(), event.peer),
                            peers_.end());
                    }
                    if (on_disconnected)
                        on_disconnected();
                    break;

                case ENET_EVENT_TYPE_NONE:
                    break;
                }
            }
        }

        Role get_role() const { return role; }
        bool is_connected() const { return peer != nullptr; }
        u8 local_player_id() const { return local_player_id_; }
        u8 max_players() const { return max_players_; }
    };
}

#endif // INCLUDE_TETRIS_NETWORK_MANAGER_HPP
