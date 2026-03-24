#ifndef INCLUDE_TETRIS_INPUT_MAPPER_HPP
#define INCLUDE_TETRIS_INPUT_MAPPER_HPP

#include <unordered_map>

#include "core/engine.hpp"

namespace tetris
{
    class InputMapper
    {
    private:
        std::unordered_map<int, Action> bindings_;

    public:
        void clear() { bindings_.clear(); }
        void bind(int key, Action action) { bindings_[key] = action; }

        bool resolve(int key, Action &out_action) const
        {
            auto it = bindings_.find(key);
            if (it == bindings_.end())
                return false;
            out_action = it->second;
            return true;
        }
    };
}

#endif // INCLUDE_TETRIS_INPUT_MAPPER_HPP
