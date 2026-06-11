#ifndef INCLUDE_TETRIS_CORE_LOCKDELAY_HPP
#define INCLUDE_TETRIS_CORE_LOCKDELAY_HPP

#include <chrono>

namespace tetris::core
{
    struct LockDelay
    {
        using Clock = std::chrono::steady_clock;

        bool active = false;
        int move_reset_count = 0;
        Clock::time_point lock_deadline{};

        static constexpr int MAX_MOVE_RESETS = 15;
        static constexpr int LOCK_DELAY_MS = 500;

        void start()
        {
            if (!active)
            {
                active = true;
                lock_deadline = Clock::now() + std::chrono::milliseconds(LOCK_DELAY_MS);
            }
        }

        void reset()
        {
            active = true;
            move_reset_count++;
            lock_deadline = Clock::now() + std::chrono::milliseconds(LOCK_DELAY_MS);
        }

        bool update()
        {
            if (!active)
                return false;

            if (Clock::now() >= lock_deadline)
            {
                active = false;
                move_reset_count = 0;
                return true;
            }
            return false;
        }

        void cancel()
        {
            active = false;
            move_reset_count = 0;
        }

        int remaining_ms() const
        {
            if (!active)
                return 0;
            auto remain = std::chrono::duration_cast<std::chrono::milliseconds>(
                lock_deadline - Clock::now()).count();
            return remain > 0 ? (int)remain : 0;
        }
    };
}

#endif
