#ifndef INCLUDE_TETRIS_INPUT_MAPPER_HPP
#define INCLUDE_TETRIS_INPUT_MAPPER_HPP

#include <unordered_map>

#include "tetris/core/engine.hpp"

namespace tetris::input
{
    using core::Action;

    class InputMapper
    {
    private:
        std::unordered_map<int, Action> m_bindings;

    public:
        void clear() { m_bindings.clear(); }
        void bind(int key, Action action) { m_bindings[key] = action; }

        bool resolve(int key, Action &out_action) const
        {
            auto it = m_bindings.find(key);
            if (it == m_bindings.end())
                return false;
            out_action = it->second;
            return true;
        }
    };
}

#endif // INCLUDE_TETRIS_INPUT_MAPPER_HPP
