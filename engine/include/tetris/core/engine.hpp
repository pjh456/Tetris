#ifndef INCLUDE_TETRIS_ENGINE_HPP
#define INCLUDE_TETRIS_ENGINE_HPP

#include <utility>

#include "tetris/core/state.hpp"
#include "tetris/core/rules.hpp"
#include "tetris/core/attack.hpp"

namespace tetris::core
{
    // 强制占 1 字节，完美契合网络发包 (3 字节大小)
    enum class Action : u8
    {
        MoveLeft,
        MoveRight,
        SoftDrop,
        HardDrop,
        RotateCW,
        RotateCCW,
        Hold
    };

    template <u8 W, u8 H>
    class Engine
    {
    public:
        State<W, H> state;
        bool game_over = false;
        bool has_hold = false;

    private:
        Piece bag[7];
        int bag_idx = 7;

        // 纯净的 LCG，保证跨平台、跨语言服务器运算时随机序列完全一致
        u32 next_rand()
        {
            state.rng = state.rng * 1664525 + 1013904223;
            return state.rng;
        }

        void shuffle_bag()
        {
            for (int i = 0; i < 7; ++i)
                bag[i] = static_cast<Piece>(i);
            for (int i = 6; i > 0; --i)
            {
                u32 j = next_rand() % (i + 1);
                std::swap(bag[i], bag[j]);
            }
        }

        Piece pop_next_piece()
        {
            if (bag_idx >= 7)
            {
                shuffle_bag();
                bag_idx = 0;
            }
            return bag[bag_idx++];
        }

        AttackResult lock_and_spawn()
        {
            int lines_cleared = lock_piece(state);
            auto attack_res = calculate_attack(state, lines_cleared);

            // 垃圾行抵消 (Canceling)
            if (attack_res.damage > 0 && state.pending_garbage > 0)
            {
                if (attack_res.damage >= state.pending_garbage)
                {
                    attack_res.damage -= state.pending_garbage;
                    state.pending_garbage = 0;
                }
                else
                {
                    state.pending_garbage -= attack_res.damage;
                    attack_res.damage = 0;
                }
            }
            else if (lines_cleared == 0 && state.pending_garbage > 0)
            {
                // 如果没有消除，且有待处理的垃圾行，则场地吃垃圾
                u8 hole_x = next_rand() % W;
                state.board.insert_garbage(state.pending_garbage, hole_x);
                state.pending_garbage = 0;
            }

            spawn(); // 内部会重置 state.last_move_was_rotation = false
            return attack_res;
        }

        void record_harddrop()
        {
            const auto &shape = PIECES[(int)state.piece].rot[(int)state.rot];
            u16 mask = 0;
            int y_min = 127;
            int y_max = -128;
            for (int i = 0; i < 4; i++)
            {
                for (int j = 0; j < 4; j++)
                {
                    if (!(shape.row[i] & (1 << j)))
                        continue;
                    int xx = state.x + j;
                    int yy = state.y + i;
                    if (xx < 0 || xx >= W)
                        continue;
                    if (yy < 0 || yy >= H)
                        continue;
                    mask |= (1u << xx);
                    if (yy < y_min)
                        y_min = yy;
                    if (yy > y_max)
                        y_max = yy;
                }
            }
            state.last_harddrop_cols = mask;
            state.last_harddrop_y_min = (y_min == 127) ? 0 : (i8)y_min;
            state.last_harddrop_y_max = (y_max == -128) ? 0 : (i8)y_max;
            state.last_harddrop_piece = state.piece;
            state.last_harddrop_valid = (mask != 0);
        }

    public:
        void reset(u32 seed = 12345)
        {
            state = State<W, H>{};
            state.rng = seed;
            game_over = false;
            has_hold = false;
            bag_idx = 7;

            for (int i = 0; i < 5; i++)
                state.next[i] = pop_next_piece();

            spawn();
        }

        void spawn()
        {
            state.piece = state.next[0];
            for (int i = 0; i < 4; i++)
                state.next[i] = state.next[i + 1];
            state.next[4] = pop_next_piece();

            state.rot = Rot::R0;
            state.x = W / 2 - 2;
            state.y = 0;
            state.hold_used = false;

            if (!can_place(state, state.x, state.y, state.rot))
                game_over = true;
        }

        // 修改为返回 AttackResult，方便向网络发送攻击包
        AttackResult handle_action(Action act)
        {
            AttackResult res{};
            if (game_over)
                return res;

            switch (act)
            {
            case Action::MoveLeft:
                _try_move_wrapped(-1, 0);
                break;
            case Action::MoveRight:
                _try_move_wrapped(1, 0);
                break;
            case Action::SoftDrop:
                _try_move_wrapped(0, 1);
                break;
            case Action::HardDrop:
            {
                hard_drop(state);
                record_harddrop();
                res = lock_and_spawn();
                break;
            }
            case Action::RotateCW:
                _try_rotate_wrapped(
                    static_cast<Rot>(((int)state.rot + 1) & 3));
                break;
            case Action::RotateCCW:
                _try_rotate_wrapped(
                    static_cast<Rot>(((int)state.rot + 3) & 3));
                break;
            case Action::Hold:
                if (!state.hold_used)
                {
                    if (has_hold)
                    {
                        Piece temp = state.hold;
                        state.hold = state.piece;
                        state.piece = temp;
                        state.rot = Rot::R0;
                        state.x = W / 2 - 2;
                        state.y = 0;
                        if (!can_place(state, state.x, state.y, state.rot))
                            game_over = true;
                    }
                    else
                    {
                        state.hold = state.piece;
                        has_hold = true;
                        spawn();
                    }
                    state.hold_used = true;
                }
                break;
            }
            return res;
        }

        // 修改为返回 AttackResult (自然下落到底也可能触发消行)
        AttackResult tick()
        {
            AttackResult res{};
            if (game_over)
                return res;

            if (!try_move(state, 0, 1))
                res = lock_and_spawn();

            return res;
        }

    private:
        bool _try_move_wrapped(int dx, int dy)
        {
            if (try_move(state, dx, dy))
            {
                state.last_move_was_rotation = false; // 平移打破旋转标记
                return true;
            }
            return false;
        }

        bool _try_rotate_wrapped(Rot to)
        {
            if (try_rotate(state, to))
            {
                state.last_move_was_rotation = true; // 记录最后一步是旋转
                return true;
            }
            return false;
        }
    };
}

#endif // INCLUDE_TETRIS_ENGINE_HPP
