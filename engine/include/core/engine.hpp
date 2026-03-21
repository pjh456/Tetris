#ifndef INCLUDE_TETRIS_ENGINE_HPP
#define INCLUDE_TETRIS_ENGINE_HPP

#include <utility>

#include "core/state.hpp"
#include "core/rules.hpp"
#include "core/attack.hpp"

namespace tetris
{
    enum class Action
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
            // 注意：因为 lock_piece 后当前方块的信息会被清除，
            // 但计算 T-spin 需要用到它，所以把整个 state 传进去
            // 我们要在清行之前进行 T-spin 检测，但具体行数要在清行后知道。
            // 幸好 check_t_spin 只看 x,y 坐标，我们在 calculate_attack 里做了。

            int lines_cleared = lock_piece(state);

            // 将整个状态扔给规则计算器
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

        void handle_action(Action act)
        {
            if (game_over)
                return;

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
                auto res = lock_and_spawn();
                break;
            }
            case Action::RotateCW:
                _try_rotate_wrapped(
                    static_cast<Rot>(
                        ((int)state.rot + 1) & 3));
                break;
            case Action::RotateCCW:
                _try_rotate_wrapped(
                    static_cast<Rot>(
                        ((int)state.rot + 3) & 3));
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
                    state.hold_used = true; // 覆盖 spawn() 中的重置
                }
                break;
            }
        }

        void tick()
        {
            if (game_over)
                return;

            if (!try_move(state, 0, 1))
                lock_and_spawn();
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