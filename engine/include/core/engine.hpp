#ifndef INCLUDE_TETRIS_ENGINE_HPP
#define INCLUDE_TETRIS_ENGINE_HPP

#include <utility>

#include "core/state.hpp"
#include "core/rules.hpp"

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

        void lock_and_spawn()
        {
            int lines = lock_piece(state);
            state.combo = (lines > 0) ? state.combo + 1 : 0;
            spawn();
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
                try_move(state, -1, 0);
                break;
            case Action::MoveRight:
                try_move(state, 1, 0);
                break;
            case Action::SoftDrop:
                try_move(state, 0, 1);
                break;
            case Action::HardDrop:
                hard_drop(state);
                lock_and_spawn();
                break;
            case Action::RotateCW:
                try_rotate(
                    state,
                    static_cast<Rot>(
                        ((int)state.rot + 1) & 3));
                break;
            case Action::RotateCCW:
                try_rotate(
                    state,
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
    };
}

#endif // INCLUDE_TETRIS_ENGINE_HPP