use std::sync::OnceLock;

#[derive(Debug, Clone, Copy)]
pub struct Kick {
    pub dx: i8,
    pub dy: i8,
}

const fn invert(k: Kick) -> Kick {
    Kick {
        dx: -k.dx,
        dy: -k.dy,
    }
}

const JLSTZ_0_1: [Kick; 5] = [
    Kick { dx: 0, dy: 0 },
    Kick { dx: -1, dy: 0 },
    Kick { dx: -1, dy: 1 },
    Kick { dx: 0, dy: -2 },
    Kick { dx: -1, dy: -2 },
];

const JLSTZ_1_2: [Kick; 5] = [
    Kick { dx: 0, dy: 0 },
    Kick { dx: 1, dy: 0 },
    Kick { dx: 1, dy: -1 },
    Kick { dx: 0, dy: 2 },
    Kick { dx: 1, dy: 2 },
];

const JLSTZ_2_3: [Kick; 5] = [
    Kick { dx: 0, dy: 0 },
    Kick { dx: 1, dy: 0 },
    Kick { dx: 1, dy: 1 },
    Kick { dx: 0, dy: -2 },
    Kick { dx: 1, dy: -2 },
];

const JLSTZ_3_0: [Kick; 5] = [
    Kick { dx: 0, dy: 0 },
    Kick { dx: -1, dy: 0 },
    Kick { dx: -1, dy: -1 },
    Kick { dx: 0, dy: 2 },
    Kick { dx: -1, dy: 2 },
];

const I_0R: [Kick; 5] = [
    Kick { dx: 0, dy: 0 },
    Kick { dx: -2, dy: 0 },
    Kick { dx: 1, dy: 0 },
    Kick { dx: -2, dy: -1 },
    Kick { dx: 1, dy: 2 },
];

const I_1R: [Kick; 5] = [
    Kick { dx: 0, dy: 0 },
    Kick { dx: -1, dy: 0 },
    Kick { dx: 2, dy: 0 },
    Kick { dx: -1, dy: 2 },
    Kick { dx: 2, dy: -1 },
];

const I_2R: [Kick; 5] = [
    Kick { dx: 0, dy: 0 },
    Kick { dx: 2, dy: 0 },
    Kick { dx: -1, dy: 0 },
    Kick { dx: 2, dy: 1 },
    Kick { dx: -1, dy: -2 },
];

const I_3R: [Kick; 5] = [
    Kick { dx: 0, dy: 0 },
    Kick { dx: 1, dy: 0 },
    Kick { dx: -2, dy: 0 },
    Kick { dx: 1, dy: -2 },
    Kick { dx: -2, dy: 1 },
];

const fn fill_pair(out: &mut [[[Kick; 5]; 4]; 4], a: usize, b: usize, base: &[Kick; 5]) {
    let mut i = 0;
    while i < 5 {
        out[a][b][i] = base[i];
        out[b][a][i] = invert(base[i]);
        i += 1;
    }
}

const fn fill_jlstz(out: &mut [[[Kick; 5]; 4]; 4]) {
    fill_pair(out, 0, 1, &JLSTZ_0_1);
    fill_pair(out, 1, 2, &JLSTZ_1_2);
    fill_pair(out, 2, 3, &JLSTZ_2_3);
    fill_pair(out, 3, 0, &JLSTZ_3_0);
}

const fn fill_i(out: &mut [[[Kick; 5]; 4]; 4]) {
    fill_pair(out, 0, 1, &I_0R);
    fill_pair(out, 1, 2, &I_1R);
    fill_pair(out, 2, 3, &I_2R);
    fill_pair(out, 3, 0, &I_3R);
}

const fn fill_o(out: &mut [[[Kick; 5]; 4]; 4]) {
    let mut a = 0;
    while a < 4 {
        let mut b = 0;
        while b < 4 {
            let mut i = 0;
            while i < 5 {
                out[a][b][i] = Kick { dx: 0, dy: 0 };
                i += 1;
            }
            b += 1;
        }
        a += 1;
    }
}

const fn make_srs() -> [[[[Kick; 5]; 4]; 4]; 7] {
    let mut table: [[[[Kick; 5]; 4]; 4]; 7] = [[[[Kick { dx: 0, dy: 0 }; 5]; 4]; 4]; 7];

    fill_i(&mut table[0]);
    fill_o(&mut table[1]);

    let mut i = 2;
    while i < 7 {
        fill_jlstz(&mut table[i]);
        i += 1;
    }

    table
}

pub static SRS: OnceLock<[[[[Kick; 5]; 4]; 4]; 7]> = OnceLock::new();

pub fn srs_table() -> &'static [[[[Kick; 5]; 4]; 4]; 7] {
    SRS.get_or_init(make_srs)
}
