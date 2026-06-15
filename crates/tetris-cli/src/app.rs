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

#[allow(dead_code)]
pub enum AppState {
    Menu {
        selected: usize,
    },
    Playing {
        engine: Engine<10, 20>,
        start_time: Instant,
        clear_flash_timer: u8,
        score_flash_timer: u8,
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
    LobbyHost {
        room_code: String,
        players: Vec<String>,
    },
    LobbyClient {
        room_code: String,
        players: Vec<String>,
    },
    PlayingMulti {
        engine: Engine<10, 20>,
        opponents: Vec<Engine<10, 20>>,
        opponent_names: Vec<String>,
        start_time: Instant,
        clear_flash_timer: u8,
        score_flash_timer: u8,
        prev_grid: [[CellType; 10]; 20],
        prev_flash_mask: u32,
        prev_half: bool,
        spectating: Option<usize>,
    },
    GameOverMulti {
        score: u32,
        lines: u32,
        level: u32,
        place: u8,
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
    #[allow(dead_code)]
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
    engine.reset_with_level(seed, 1);
    AppState::Playing {
        engine,
        start_time: Instant::now(),
        clear_flash_timer: 0,
        score_flash_timer: 0,
        prev_grid: [[CellType::Empty; 10]; 20],
        prev_flash_mask: 0,
        prev_half: false,
    }
}

fn step(state: AppState, msg: Message) -> (AppState, bool) {
    match state {
        AppState::Menu { mut selected } => match msg {
            Message::Key(KeyCode::Up) => {
                selected = selected.saturating_sub(1);
                (AppState::Menu { selected }, false)
            }
            Message::Key(KeyCode::Down) => {
                if selected < 5 {
                    selected += 1;
                }
                (AppState::Menu { selected }, false)
            }
            Message::Key(KeyCode::Enter) => match selected {
                0 => (start_game(), false),
                1 => (
                    AppState::LobbyHost {
                        room_code: String::new(),
                        players: vec!["Host".into()],
                    },
                    false,
                ),
                2 => (
                    AppState::LobbyClient {
                        room_code: String::new(),
                        players: vec!["Joining...".into()],
                    },
                    false,
                ),
                3 => (
                    AppState::LobbyClient {
                        room_code: String::new(),
                        players: vec!["Joining relay...".into()],
                    },
                    false,
                ),
                4 => (AppState::Menu { selected }, false),
                5 => (AppState::Menu { selected }, true),
                _ => (AppState::Menu { selected }, false),
            },
            Message::Key(KeyCode::Char('q')) => (AppState::Menu { selected }, true),
            Message::Quit => (AppState::Menu { selected }, true),
            _ => (AppState::Menu { selected }, false),
        },

        AppState::LobbyHost { room_code, players } => match msg {
            Message::Key(_) => (AppState::Menu { selected: 1 }, false),
            Message::Quit => (AppState::Menu { selected: 0 }, true),
            _ => (AppState::LobbyHost { room_code, players }, false),
        },

        AppState::LobbyClient { room_code, players } => match msg {
            Message::Key(_) => (AppState::Menu { selected: 2 }, false),
            Message::Quit => (AppState::Menu { selected: 0 }, true),
            _ => (AppState::LobbyClient { room_code, players }, false),
        },

        AppState::Playing {
            mut engine,
            start_time,
            mut clear_flash_timer,
            mut score_flash_timer,
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
                    return (AppState::Pause { engine, start_time }, false);
                }
                Message::Tick => {
                    engine.scorer.tick_time(20);
                    let prev_score = engine.scorer.score;
                    engine.tick(20);
                    if engine.scorer.score != prev_score {
                        score_flash_timer = 10;
                    }
                    if engine.state.last_clear_count > 0 {
                        clear_flash_timer = 8;
                    }
                }
                Message::FrameTick => {
                    clear_flash_timer = clear_flash_timer.saturating_sub(1);
                    score_flash_timer = score_flash_timer.saturating_sub(1);
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
                    prev_grid,
                    prev_flash_mask,
                    prev_half,
                },
                false,
            )
        }

        AppState::Pause { engine, start_time } => match msg {
            Message::Key(KeyCode::Char('p')) => (
                AppState::Playing {
                    engine,
                    start_time,
                    clear_flash_timer: 0,
                    score_flash_timer: 0,
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

        AppState::GameOverMulti {
            score,
            lines,
            level,
            place,
        } => match msg {
            Message::Key(_) => (AppState::Menu { selected: 0 }, false),
            Message::Quit => (AppState::Menu { selected: 0 }, true),
            _ => (
                AppState::GameOverMulti {
                    score,
                    lines,
                    level,
                    place,
                },
                false,
            ),
        },

        AppState::PlayingMulti {
            mut engine,
            opponents,
            opponent_names,
            start_time,
            clear_flash_timer,
            score_flash_timer,
            prev_grid,
            prev_flash_mask,
            prev_half,
            spectating,
        } => match msg {
            Message::Tick => {
                engine.scorer.tick_time(20);
                engine.tick(20);
                (
                    AppState::PlayingMulti {
                        engine,
                        opponents,
                        opponent_names,
                        start_time,
                        clear_flash_timer: clear_flash_timer.saturating_sub(1),
                        score_flash_timer: score_flash_timer.saturating_sub(1),
                        prev_grid,
                        prev_flash_mask,
                        prev_half,
                        spectating,
                    },
                    false,
                )
            }
            Message::FrameTick => (
                AppState::PlayingMulti {
                    engine,
                    opponents,
                    opponent_names,
                    start_time,
                    clear_flash_timer: clear_flash_timer.saturating_sub(1),
                    score_flash_timer: score_flash_timer.saturating_sub(1),
                    prev_grid,
                    prev_flash_mask,
                    prev_half,
                    spectating,
                },
                false,
            ),
            Message::Quit => (AppState::Menu { selected: 0 }, true),
            Message::Key(k) => {
                let action = match k {
                    KeyCode::Left | KeyCode::Char('a') => Action::MoveLeft,
                    KeyCode::Right | KeyCode::Char('d') => Action::MoveRight,
                    KeyCode::Down | KeyCode::Char('s') => Action::SoftDrop,
                    KeyCode::Up | KeyCode::Char('w') => Action::RotateCW,
                    KeyCode::Char('z') => Action::RotateCCW,
                    KeyCode::Char(' ') => Action::HardDrop,
                    KeyCode::Tab => Action::Hold,
                    KeyCode::Char('p') => return (AppState::Pause { engine, start_time }, false),
                    _ => return (
                        AppState::PlayingMulti {
                            engine, opponents, opponent_names, start_time,
                            clear_flash_timer, score_flash_timer,
                            prev_grid, prev_flash_mask, prev_half,
                            spectating,
                        },
                        false,
                    ),
                };
                engine.handle_action(action);
                (
                    AppState::PlayingMulti {
                        engine, opponents, opponent_names, start_time,
                        clear_flash_timer, score_flash_timer,
                        prev_grid, prev_flash_mask, prev_half,
                        spectating,
                    },
                    false,
                )
            }
            _ => (
                AppState::PlayingMulti {
                    engine,
                    opponents,
                    opponent_names,
                    start_time,
                    clear_flash_timer,
                    score_flash_timer,
                    prev_grid,
                    prev_flash_mask,
                    prev_half,
                    spectating,
                },
                false,
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_core::engine::gravity_interval_ms;

    #[test]
    fn test_gravity_level_1() {
        let ms = gravity_interval_ms(1);
        assert_eq!(ms, 800, "level 1: {ms}ms");
    }

    #[test]
    fn test_gravity_level_15() {
        let ms = gravity_interval_ms(15);
        assert!(ms <= 8, "level 15: {ms}ms");
    }

    #[test]
    fn test_gravity_monotonic_decreasing() {
        for lvl in 1..15 {
            assert!(
                gravity_interval_ms(lvl) > gravity_interval_ms(lvl + 1),
                "level {lvl} not > level {}",
                lvl + 1
            );
        }
    }

    #[test]
    fn test_gravity_min_floor() {
        assert!(gravity_interval_ms(100) >= 1);
    }

    #[test]
    fn test_menu_navigate_down() {
        let (next, quit) = step(AppState::Menu { selected: 0 }, Message::Key(KeyCode::Down));
        assert!(!quit);
        match next {
            AppState::Menu { selected } => assert_eq!(selected, 1),
            _ => panic!("expected Menu"),
        }
    }

    #[test]
    fn test_menu_navigate_up_clamp() {
        let (next, _) = step(AppState::Menu { selected: 0 }, Message::Key(KeyCode::Up));
        match next {
            AppState::Menu { selected } => assert_eq!(selected, 0),
            _ => panic!("expected Menu"),
        }
    }

    #[test]
    fn test_menu_enter_starts_game() {
        let (next, quit) = step(AppState::Menu { selected: 0 }, Message::Key(KeyCode::Enter));
        assert!(!quit);
        assert!(matches!(next, AppState::Playing { .. }));
    }

    #[test]
    fn test_menu_quit_index_5() {
        let (_, quit) = step(AppState::Menu { selected: 5 }, Message::Key(KeyCode::Enter));
        assert!(quit);
    }

    #[test]
    fn test_playing_to_pause() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let state = AppState::Playing {
            engine,
            start_time: Instant::now(),
            clear_flash_timer: 0,
            score_flash_timer: 0,
            prev_grid: [[CellType::Empty; 10]; 20],
            prev_flash_mask: 0,
            prev_half: false,
        };
        let (next, quit) = step(state, Message::Key(KeyCode::Char('p')));
        assert!(!quit);
        assert!(matches!(next, AppState::Pause { .. }));
    }

    #[test]
    fn test_pause_to_playing() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let state = AppState::Pause {
            engine,
            start_time: Instant::now(),
        };
        let (next, quit) = step(state, Message::Key(KeyCode::Char('p')));
        assert!(!quit);
        assert!(matches!(next, AppState::Playing { .. }));
    }

    #[test]
    fn test_gameover_enter_returns_menu() {
        let state = AppState::GameOver {
            score: 1000,
            lines: 10,
            level: 2,
            max_combo: 3,
            tspin_count: 1,
        };
        let (next, quit) = step(state, Message::Key(KeyCode::Enter));
        assert!(!quit);
        assert!(matches!(next, AppState::Menu { .. }));
    }

    #[test]
    fn test_update_quit_signal() {
        let mut state = AppState::Menu { selected: 0 };
        let quit = update(&mut state, Message::Quit);
        assert!(quit);
    }
}
