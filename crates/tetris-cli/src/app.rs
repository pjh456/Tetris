use crossterm::event::KeyCode;
use std::time::Instant;
use tetris_core::engine::{Action, Engine};
use tetris_core::types::Piece;

#[derive(Clone, Copy, PartialEq)]
pub enum CellType {
    Empty,
    Locked,
    Ghost,
    Active(Piece),
}

fn gravity_interval_ms(level: u32) -> u32 {
    let lvl = level.max(1) as f64;
    let seconds = (0.8 - (lvl - 1.0) * 0.007).powf(lvl - 1.0);
    (seconds * 1000.0).max(1.0) as u32
}

pub enum AppState {
    Menu {
        selected: usize,
    },
    Lobby,
    Playing {
        engine: Engine<10, 20>,
        start_time: Instant,
        clear_flash_timer: u8,
        score_flash_timer: u8,
        gravity_accum_ms: u32,
        prev_grid: [[CellType; 10]; 20],
        prev_flash_mask: u32,
        prev_half: bool,
    },
    Pause {
        engine: Engine<10, 20>,
        start_time: Instant,
    },
    GameOver {
        score: u32,
        lines: u32,
        level: u32,
        max_combo: u32,
        tspin_count: u32,
    },
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Menu { selected: 0 }
    }
}

pub enum Message {
    Key(KeyCode),
    Tick,
    FrameTick,
    Quit,
}

pub fn update(state: &mut AppState, msg: Message) -> bool {
    let current = std::mem::take(state);
    let (next, quit) = step(current, msg);
    *state = next;
    quit
}

fn start_game() -> AppState {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32;
    let mut engine = Engine::<10, 20>::new();
    engine.reset(seed);
    AppState::Playing {
        engine,
        start_time: Instant::now(),
        clear_flash_timer: 0,
        score_flash_timer: 0,
        gravity_accum_ms: 0,
        prev_grid: [[CellType::Empty; 10]; 20],
        prev_flash_mask: 0,
        prev_half: false,
    }
}

fn step(state: AppState, msg: Message) -> (AppState, bool) {
    match state {
        AppState::Menu { mut selected } => match msg {
            Message::Key(KeyCode::Up) => {
                if selected > 0 {
                    selected -= 1;
                }
                (AppState::Menu { selected }, false)
            }
            Message::Key(KeyCode::Down) => {
                if selected < 3 {
                    selected += 1;
                }
                (AppState::Menu { selected }, false)
            }
            Message::Key(KeyCode::Enter) => match selected {
                0 => (start_game(), false),
                1 => (AppState::Lobby, false),
                3 => (AppState::Menu { selected }, true),
                _ => (AppState::Menu { selected }, false),
            },
            Message::Key(KeyCode::Char('q')) => (AppState::Menu { selected }, true),
            Message::Quit => (AppState::Menu { selected }, true),
            _ => (AppState::Menu { selected }, false),
        },

        AppState::Lobby => match msg {
            Message::Key(_) => (AppState::Menu { selected: 0 }, false),
            Message::Quit => (AppState::Menu { selected: 0 }, true),
            _ => (AppState::Lobby, false),
        },

        AppState::Playing {
            mut engine,
            start_time,
            mut clear_flash_timer,
            mut score_flash_timer,
            mut gravity_accum_ms,
            prev_grid,
            prev_flash_mask,
            prev_half,
        } => {
            match msg {
                Message::Key(KeyCode::Left) => {
                    engine.handle_action(Action::MoveLeft);
                }
                Message::Key(KeyCode::Right) => {
                    engine.handle_action(Action::MoveRight);
                }
                Message::Key(KeyCode::Down) => {
                    engine.handle_action(Action::SoftDrop);
                }
                Message::Key(KeyCode::Up) => {
                    engine.handle_action(Action::RotateCW);
                }
                Message::Key(KeyCode::Char('z')) => {
                    engine.handle_action(Action::RotateCCW);
                }
                Message::Key(KeyCode::Char('x')) => {
                    engine.handle_action(Action::RotateCW);
                }
                Message::Key(KeyCode::Char(' ')) => {
                    let prev_score = engine.scorer.score;
                    engine.handle_action(Action::HardDrop);
                    if engine.scorer.score != prev_score {
                        score_flash_timer = 10;
                    }
                    if engine.state.last_clear_count > 0 {
                        clear_flash_timer = 8;
                    }
                }
                Message::Key(KeyCode::Tab) => {
                    engine.handle_action(Action::Hold);
                }
                Message::Key(KeyCode::Char('p')) => {
                    return (
                        AppState::Pause {
                            engine,
                            start_time,
                        },
                        false,
                    );
                }
                Message::Tick => {
                    gravity_accum_ms += 20;
                    engine.scorer.tick_time(20);

                    let on_surface = !tetris_core::rules::can_place(
                        &engine.state,
                        engine.state.x,
                        engine.state.y + 1,
                        engine.state.rot,
                    );

                    let interval = gravity_interval_ms(engine.scorer.level);
                    let should_tick = if on_surface {
                        true
                    } else {
                        gravity_accum_ms >= interval
                    };

                    if should_tick {
                        let prev_score = engine.scorer.score;
                        engine.tick();
                        if !on_surface {
                            gravity_accum_ms = 0;
                        }
                        if engine.scorer.score != prev_score {
                            score_flash_timer = 10;
                        }
                        if engine.state.last_clear_count > 0 {
                            clear_flash_timer = 8;
                        }
                    }
                }
                Message::FrameTick => {
                    if clear_flash_timer > 0 {
                        clear_flash_timer -= 1;
                    }
                    if score_flash_timer > 0 {
                        score_flash_timer -= 1;
                    }
                }
                Message::Quit => return (AppState::Menu { selected: 0 }, true),
                _ => {}
            }

            if engine.game_over {
                let s = &engine.scorer;
                return (
                    AppState::GameOver {
                        score: s.score,
                        lines: s.total_lines,
                        level: s.level,
                        max_combo: s.max_combo,
                        tspin_count: s.tspin_count,
                    },
                    false,
                );
            }

            (
                AppState::Playing {
                    engine,
                    start_time,
                    clear_flash_timer,
                    score_flash_timer,
                    gravity_accum_ms,
                    prev_grid,
                    prev_flash_mask,
                    prev_half,
                },
                false,
            )
        }

        AppState::Pause {
            engine,
            start_time,
        } => match msg {
            Message::Key(KeyCode::Char('p')) => (
                AppState::Playing {
                    engine,
                    start_time,
                    clear_flash_timer: 0,
                    score_flash_timer: 0,
                    gravity_accum_ms: 0,
                    prev_grid: [[CellType::Empty; 10]; 20],
                    prev_flash_mask: 0,
                    prev_half: false,
                },
                false,
            ),
            Message::Key(KeyCode::Char('q')) => {
                let s = &engine.scorer;
                (
                    AppState::GameOver {
                        score: s.score,
                        lines: s.total_lines,
                        level: s.level,
                        max_combo: s.max_combo,
                        tspin_count: s.tspin_count,
                    },
                    false,
                )
            }
            Message::Quit => (AppState::Menu { selected: 0 }, true),
            _ => (AppState::Pause { engine, start_time }, false),
        },

        AppState::GameOver {
            score,
            lines,
            level,
            max_combo,
            tspin_count,
        } => match msg {
            Message::Key(KeyCode::Char('q')) => (
                AppState::GameOver {
                    score,
                    lines,
                    level,
                    max_combo,
                    tspin_count,
                },
                true,
            ),
            Message::Key(KeyCode::Enter) => (AppState::Menu { selected: 0 }, false),
            Message::Key(_) => (AppState::Menu { selected: 0 }, false),
            Message::Quit => (AppState::Menu { selected: 0 }, true),
            _ => (
                AppState::GameOver {
                    score,
                    lines,
                    level,
                    max_combo,
                    tspin_count,
                },
                false,
            ),
        },
    }
}
