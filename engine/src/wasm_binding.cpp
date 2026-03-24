#include <emscripten/bind.h>
#include <vector>
#include "tetris/core/types.hpp"
#include "tetris/core/session.hpp"
#include "tetris/core/rules.hpp"

using namespace emscripten;
using namespace tetris::core;

class WebTetris
{
private:
    GameSession<10, 20> session;
    std::vector<int> grid_buffer; // 用于传递给前端的 10x20 数组
    std::vector<int> next_buffer; // 5 个 Next 方块
    bool has_hold = false;

public:
    WebTetris(uint32_t seed)
    {
        session.reset(seed);
        grid_buffer.resize(10 * 20);
        next_buffer.resize(5);
        has_hold = false;
    }

    void reset(uint32_t seed)
    {
        session.reset(seed);
        has_hold = false;
    }
    void tick() { session.tick(); }
    void handleAction(int action_val)
    {
        auto act = static_cast<Action>(action_val);
        session.handle_action(act);
        if (act == Action::Hold)
            has_hold = true;
    }
    bool isGameOver() const { return session.is_game_over(); }

    // render 逻辑简化，生成一个数字网格交由 JS 渲染
    val getGrid()
    {
        std::fill(grid_buffer.begin(), grid_buffer.end(), 0);
        const auto &s = session.state();

        // 1. 填入已锁定方块 (值为 1)
        for (int y = 0; y < 20; y++)
        {
            for (int x = 0; x < 10; x++)
            {
                if ((s.board.rows[y] >> x) & 1)
                    grid_buffer[y * 10 + x] = 1;
            }
        }

        if (!session.is_game_over())
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

    int getHold()
    {
        if (!has_hold)
            return -1;
        return static_cast<int>(session.state().hold);
    }

    val getNext()
    {
        const auto &s = session.state();
        for (int i = 0; i < 5; ++i)
            next_buffer[i] = static_cast<int>(s.next[i]);
        return val(typed_memory_view(next_buffer.size(), next_buffer.data()));
    }

    bool wouldHitWall(int dx)
    {
        const auto &s = session.state();
        const auto &shape = PIECES[(int)s.piece].rot[(int)s.rot];
        for (int i = 0; i < 4; i++)
        {
            for (int j = 0; j < 4; j++)
            {
                if (!(shape.row[i] & (1 << j)))
                    continue;
                int nx = s.x + j + dx;
                if (nx < 0 || nx >= 10)
                    return true;
            }
        }
        return false;
    }

    bool canMove(int dx)
    {
        const auto &s = session.state();
        return can_place(s, s.x + dx, s.y, s.rot);
    }

    u32 getLastClearMask()
    {
        auto &s = session.state();
        u32 mask = s.last_clear_mask;
        s.last_clear_mask = 0;
        return mask;
    }

    u8 getLastClearCount()
    {
        auto &s = session.state();
        u8 count = s.last_clear_count;
        s.last_clear_count = 0;
        return count;
    }

    val getLastHardDropInfo()
    {
        auto &s = session.state();
        if (!s.last_harddrop_valid)
            return val::null();
        val arr = val::array();
        arr.set(0, (int)s.last_harddrop_cols);
        arr.set(1, (int)s.last_harddrop_start_y);
        arr.set(2, (int)s.last_harddrop_end_y);
        arr.set(3, (int)s.last_harddrop_piece);
        s.last_harddrop_valid = false;
        return arr;
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
        .function("getGrid", &WebTetris::getGrid)
        .function("getHold", &WebTetris::getHold)
        .function("getNext", &WebTetris::getNext)
        .function("wouldHitWall", &WebTetris::wouldHitWall)
        .function("canMove", &WebTetris::canMove)
        .function("getLastClearMask", &WebTetris::getLastClearMask)
        .function("getLastClearCount", &WebTetris::getLastClearCount)
        .function("getLastHardDropInfo", &WebTetris::getLastHardDropInfo);
}
