// --- START OF FILE attack.hpp ---
#ifndef INCLUDE_TETRIS_ATTACK_HPP
#define INCLUDE_TETRIS_ATTACK_HPP

#include "core/types.hpp"
#include "core/state.hpp"

namespace tetris
{
    // 用于通知 UI 播放特效和网络发送的结算结果
    struct AttackResult
    {
        int damage;         // 发送给对手的垃圾行
        bool is_tspin;      // 是否触发 T-Spin
        bool is_mini;       // 是否为 Mini T-Spin (可选)
        bool is_b2b;        // 本次是否享受 B2B 加成
        bool perfect_clear; // 是否全清
    };

    // 检查场地上某个点是否被占据（墙壁和地板也算占据，用于 T-Spin 检测）
    template <u8 W, u8 H>
    bool is_occupied(const Board<W, H> &board, int x, int y)
    {
        if (x < 0 || x >= W || y >= H || y < 0)
            return true; // 越界视为被墙壁/地板占据
        return (board.rows[y] & (1ULL << x)) != 0;
    }

    // 核心 T-Spin 检测 (3角判定法)
    template <u8 W, u8 H>
    bool check_t_spin(const State<W, H> &st)
    {
        // 必须是 T 方块，且最后一步必须是旋转
        if (st.piece != Piece::T || !st.last_move_was_rotation)
            return false;

        // T 方块的 3x3 矩阵中心在局部坐标 (1, 1)
        int cx = st.x + 1;
        int cy = st.y + 1;

        int corners = 0;
        if (is_occupied(st.board, cx - 1, cy - 1))
            corners++; // 左上
        if (is_occupied(st.board, cx + 1, cy - 1))
            corners++; // 右上
        if (is_occupied(st.board, cx - 1, cy + 1))
            corners++; // 左下
        if (is_occupied(st.board, cx + 1, cy + 1))
            corners++; // 右下

        // 现代规则：4 个角中有 3 个及以上被占据，即为 T-Spin
        return corners >= 3;
    }

    // 计算标准现代俄罗斯方块 (Guideline) 的攻击伤害
    template <u8 W, u8 H>
    AttackResult
    calculate_attack(
        State<W, H> &st,
        int lines_cleared)
    {
        AttackResult res{};
        if (lines_cleared == 0)
        {
            st.combo = 0;
            return res; // 没有消除，直接打断 Combo 并返回
        }

        res.is_tspin = check_t_spin(st);

        // 1. 基础伤害判定
        if (res.is_tspin)
        {
            // T-Spin 伤害表: 1行->2, 2行->4, 3行->6
            const int tspin_dmg[] = {0, 2, 4, 6, 8};
            res.damage = tspin_dmg[lines_cleared];
        }
        else
        {
            // 普通消除伤害表: 1行->0, 2行->1, 3行->2, 4行->4
            const int normal_dmg[] = {0, 0, 1, 2, 4};
            res.damage = normal_dmg[lines_cleared];
        }

        // 2. Back-to-Back (B2B) 判定
        // 触发条件：本次是 Tetris (消除4行) 或者是 T-Spin，并且上一次也是
        bool is_difficult_clear = (lines_cleared == 4 || res.is_tspin);
        if (is_difficult_clear)
        {
            if (st.b2b)
            {
                res.damage += 1; // 连续的高难度消除，额外 +1 行伤害
                res.is_b2b = true;
            }
            st.b2b = true; // 保持或激活 B2B 状态
        }
        else
        {
            st.b2b = false; // 普通 1/2/3 行消除会断掉 B2B
        }

        // 3. Combo 加成计算
        // 现代 Combo 伤害递增表 (0次无加成，第1/2次+1，第3/4次+2...)
        const int combo_dmg[] = {0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 4, 5};
        int combo_idx = std::min(st.combo, (int)(sizeof(combo_dmg) / sizeof(combo_dmg[0]) - 1));
        res.damage += combo_dmg[combo_idx];
        st.combo++; // 更新 Combo 计数器

        // 4. 完美消除 (Perfect Clear)
        res.perfect_clear = st.board.is_empty();
        if (res.perfect_clear)
        {
            res.damage += 10; // PC 直接糊对面 10 行
        }

        return res;
    }
}

#endif // INCLUDE_TETRIS_ATTACK_HPP