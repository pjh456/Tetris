#ifndef INCLUDE_TETRIS_PROTOCOL_HPP
#define INCLUDE_TETRIS_PROTOCOL_HPP

#include "core/types.hpp"
#include "core/engine.hpp"

namespace tetris::net
{
    // 网络包类型枚举
    enum class PacketType : u8
    {
        ClientJoin = 1, // 客机请求加入
        ServerAccept,   // 主机同意加入，分配玩家ID
        GameStart,      // 游戏开始 (下发随机数种子)
        PlayerAction,   // 玩家输入操作 (极低开销)
        PlayerAttack,   // 玩家发送垃圾行 (可靠到达)
        StateSync,      // 完整状态同步 (兜底防断线/错位)
        GameOver        // 玩家死亡通知
    };

    // 强制 1 字节对齐，避免网络传输时产生内存空洞
#pragma pack(push, 1)

    // 所有数据包的通用头部 (共 2 字节)
    struct PacketHeader
    {
        PacketType type;
        u8 player_id; // 发送者的身份：0 为 Host，1 为 Client
    };

    // 1. 游戏开始包 (共 6 字节) - 必须走可靠通道 (Reliable)
    struct PktGameStart
    {
        PacketHeader header;
        u32 random_seed; // 保证双方发牌序列完全一致的核心
    };

    // 2. 玩家操作包 (共 3 字节) - 走可靠或有序不可靠通道
    struct PktPlayerAction
    {
        PacketHeader header;
        Action action; // 执行的动作
    };

    // 3. 攻击包 (共 4 字节) - 必须走可靠通道
    struct PktPlayerAttack
    {
        PacketHeader header;
        u8 lines;  // 发送给对手的垃圾行数量
        u8 hole_x; // 垃圾行的缺口位置 (保证双方看到的垃圾行缺口一致)
    };

    // 4. 状态同步包 (开销视 H 而定，H=20 时约 170 字节) - 走不可靠通道 (Unreliable)，每秒发送 10~20 次
    template <u8 W, u8 H>
    struct PktStateSync
    {
        PacketHeader header;
        u64 board_rows[H]; // 完整的场地数据
        Piece piece;       // 当前操控的方块
        Rot rot;           // 当前旋转状态
        i8 x;              // 坐标 X
        i8 y;              // 坐标 Y
        Piece hold;        // Hold 槽的方块
        bool hold_used;
        Piece next[3];
        u8 pending_garbage; // 正在排队等待进入场地的垃圾行
        u32 rng_state;      // 当前随机数状态，用于严格校验
    };

    // 5. 游戏结束包 (共 2 字节) - 可靠通道
    struct PktGameOver
    {
        PacketHeader header;
    };

#pragma pack(pop)

}

#endif // INCLUDE_TETRIS_PROTOCOL_HPP