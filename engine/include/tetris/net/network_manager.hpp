// --- START OF FILE network_manager.hpp ---
#ifndef INCLUDE_TETRIS_NETWORK_MANAGER_HPP
#define INCLUDE_TETRIS_NETWORK_MANAGER_HPP

#include <enet/enet.h>
#include <functional>
#include <iostream>
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
        ENetPeer *peer = nullptr; // 1v1 对战中，我们只需要记录对面的 peer
        Role role = Role::None;

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
        bool start_server(uint16_t port)
        {
            ENetAddress address;
            address.host = ENET_HOST_ANY;
            address.port = port;

            // 参数: 地址, 最大连接数(1v1所以是1), 通道数(2), 入带宽(0=不限), 出带宽(0=不限)
            host = enet_host_create(&address, 1, 2, 0, 0);
            if (!host)
                return false;

            role = Role::Host;
            std::cout << "Server started on port " << port << std::endl;
            return true;
        }

        // --- 作为客机连接 (连接到指定IP) ---
        bool connect_to_server(const char *ip, uint16_t port)
        {
            // 客机不需要绑定特定端口，传 NULL
            host = enet_host_create(NULL, 1, 2, 0, 0);
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
                        PktGameStart start_pkt;
                        start_pkt.header = {PacketType::GameStart, 0};
                        start_pkt.random_seed = 202405; // MVP 先写死或者用 time()

                        send_packet(start_pkt, 0, true);

                        // Server 本地也触发 GameStart
                        if (on_game_start)
                            on_game_start(start_pkt.random_seed);
                    }
                    break;

                case ENET_EVENT_TYPE_RECEIVE:
                    // 将收到的数据抛给外层游戏逻辑处理
                    if (on_packet_received)
                    {
                        on_packet_received(event.packet->data, event.packet->dataLength);
                    }
                    // ENet 要求手动销毁收到的 packet
                    enet_packet_destroy(event.packet);
                    break;

                case ENET_EVENT_TYPE_DISCONNECT:
                    std::cout << "Peer disconnected." << std::endl;
                    peer = nullptr;
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
    };
}

#endif // INCLUDE_TETRIS_NETWORK_MANAGER_HPP