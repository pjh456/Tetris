#include <emscripten/bind.h>
#include <vector>
#include "core/types.hpp"
#include "core/engine.hpp"

using namespace emscripten;
using namespace tetris;

class WebTetris
{
private:
    Engine<10, 20> engine;
    std::vector<int> grid_buffer; // 用于传递给前端的 10x20 数组

public:
    WebTetris(uint32_t seed)
    {
        engine.reset(seed);
        grid_buffer.resize(10 * 20);
    }

    void reset(uint32_t seed) { engine.reset(seed); }
    void tick() { engine.tick(); }
    void handleAction(int action_val) { engine.handle_action(static_cast<Action>(action_val)); }
    bool isGameOver() const { return engine.game_over; }

    // 将你原本在 main.cpp 里的 render 逻辑简化，生成一个数字网格交由 JS 渲染
    val getGrid()
    {
        std::fill(grid_buffer.begin(), grid_buffer.end(), 0);
        const auto &s = engine.state;

        // 1. 填入已锁定方块 (值为 1)
        for (int y = 0; y < 20; y++)
        {
            for (int x = 0; x < 10; x++)
            {
                if ((s.board.rows[y] >> x) & 1)
                    grid_buffer[y * 10 + x] = 1;
            }
        }

        if (!engine.game_over)
        {
            int ghost_y = get_ghost_y(s);
            const auto &shape = PIECES[(int)s.piece].rot[(int)s.rot];

            // 2. 填入 Ghost (值为 2)
            for (int i = 0; i < 4; i++)
            {
                for (int j = 0; j < 4; j++)
                {
                    if (shape.row[i] & (1 << j))
                    {
                        int xx = s.x + j, yy = ghost_y + i;
                        if (yy >= 0 && yy < 20 && xx >= 0 && xx < 10)
                            grid_buffer[yy * 10 + xx] = 2;
                    }
                }
            }
            // 3. 填入 当前方块 (用具体形状的 enum 值或固定的 3 区分颜色)
            for (int i = 0; i < 4; i++)
            {
                for (int j = 0; j < 4; j++)
                {
                    if (shape.row[i] & (1 << j))
                    {
                        int xx = s.x + j, yy = s.y + i;
                        if (yy >= 0 && yy < 20 && xx >= 0 && xx < 10)
                            grid_buffer[yy * 10 + xx] = 3 + (int)s.piece;
                    }
                }
            }
        }

        // 转化为 JS 的 TypedArray，性能极高
        return val(typed_memory_view(grid_buffer.size(), grid_buffer.data()));
    }
};

// 导出模块
EMSCRIPTEN_BINDINGS(tetris_module)
{
    class_<WebTetris>("WebTetris")
        .constructor<uint32_t>()
        .function("reset", &WebTetris::reset)
        .function("tick", &WebTetris::tick)
        .function("handleAction", &WebTetris::handleAction)
        .function("isGameOver", &WebTetris::isGameOver)
        .function("getGrid", &WebTetris::getGrid);
}