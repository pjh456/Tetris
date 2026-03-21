#ifndef INCLUDE_TETRIS_SRS_HPP
#define INCLUDE_TETRIS_SRS_HPP

#include "core/types.hpp"

namespace tetris
{
    struct Kick
    {
        i8 dx, dy;
    };

    struct KickTable
    {
        Kick data[7][4][4][5];
    };

    constexpr Kick invert(Kick k)
    {
        return {(i8)-k.dx, (i8)-k.dy};
    }

    constexpr Kick JLSTZ_0_1[5] =
        {{0, 0}, {-1, 0}, {-1, 1}, {0, -2}, {-1, -2}};
    constexpr Kick JLSTZ_1_2[5] =
        {{0, 0}, {1, 0}, {1, -1}, {0, 2}, {1, 2}};
    constexpr Kick JLSTZ_2_3[5] =
        {{0, 0}, {1, 0}, {1, 1}, {0, -2}, {1, -2}};
    constexpr Kick JLSTZ_3_0[5] =
        {{0, 0}, {-1, 0}, {-1, -1}, {0, 2}, {-1, 2}};
    constexpr Kick I_0R[5] = {
        {0, 0}, {-2, 0}, {1, 0}, {-2, -1}, {1, 2}};

    constexpr void fill_pair(
        Kick out[4][4][5],
        int a,
        int b,
        const Kick base[5])
    {
        for (int i = 0; i < 5; i++)
        {
            out[a][b][i] = base[i];
            out[b][a][i] = invert(base[i]);
        }
    }

    constexpr void fill_jlstz(Kick out[4][4][5])
    {
        fill_pair(out, 0, 1, JLSTZ_0_1);
        fill_pair(out, 1, 2, JLSTZ_1_2);
        fill_pair(out, 2, 3, JLSTZ_2_3);
        fill_pair(out, 3, 0, JLSTZ_3_0);
    }

    constexpr void fill_I(Kick out[4][4][5])
    {
        fill_pair(out, 0, 1, I_0R);
        fill_pair(out, 1, 2, I_0R);
        fill_pair(out, 2, 3, I_0R);
        fill_pair(out, 3, 0, I_0R);
    }

    constexpr void fill_O(Kick out[4][4][5])
    {
        for (int a = 0; a < 4; a++)
            for (int b = 0; b < 4; b++)
                for (int i = 0; i < 5; i++)
                    out[a][b][i] = {0, 0};
    }

    constexpr KickTable make_srs()
    {
        KickTable table{};

        // I=0 O=1 T=2 S=3 Z=4 J=5 L=6

        fill_I(table.data[0]);
        fill_O(table.data[1]);

        for (int i = 2; i < 7; i++)
            fill_jlstz(table.data[i]);

        return table;
    }

    constexpr auto SRS = make_srs();
}

#endif // INCLUDE_TETRIS_SRS_HPP