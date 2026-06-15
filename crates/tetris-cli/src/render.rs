use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use tetris_core::piece::PIECES;
use tetris_core::rules::get_ghost_y;
use tetris_core::types::Piece;

use crate::app::AppState;

const PIECE_COLORS: [Color; 7] = [
    Color::Cyan,
    Color::Yellow,
    Color::Magenta,
    Color::Green,
    Color::Red,
    Color::Blue,
    Color::Rgb(255, 136, 0),
];

fn piece_color(p: Piece) -> Color {
    PIECE_COLORS[p as usize]
}

pub fn render(state: &mut AppState, frame: &mut Frame) {
    match state {
        AppState::Menu { selected } => render_menu(frame, *selected),
        AppState::LobbyHost { room_code, players } => render_lobby_host(frame, room_code, players),
        AppState::LobbyClient { room_code, players } => {
            render_lobby_client(frame, room_code, players)
        }
        AppState::Playing {
            engine,
            clear_flash_timer,
            score_flash_timer,
            prev_grid,
            prev_flash_mask,
            prev_half,
            ..
        } => render_playing(
            frame,
            engine,
            *clear_flash_timer,
            *score_flash_timer,
            prev_grid,
            prev_flash_mask,
            prev_half,
        ),
        AppState::Pause { .. } => render_pause(frame),
        AppState::GameOver {
            score,
            lines,
            level,
            max_combo,
            tspin_count,
        } => render_game_over(frame, *score, *lines, *level, *max_combo, *tspin_count),
        AppState::GameOverMulti {
            score,
            lines,
            level,
            place,
        } => {
            render_game_over_multi(frame, *score, *lines, *level, *place);
        }
        AppState::PlayingMulti {
            engine,
            opponents,
            opponent_names,
            clear_flash_timer,
            score_flash_timer,
            prev_grid,
            prev_flash_mask,
            prev_half,
            spectating,
            ..
        } => render_multi(
            frame,
            engine,
            opponents,
            opponent_names,
            *clear_flash_timer,
            *score_flash_timer,
            prev_grid,
            prev_flash_mask,
            prev_half,
            *spectating,
        ),
    }
}

fn render_menu(frame: &mut Frame, selected: usize) {
    let area = frame.area();

    let ascii_art = vec![
        "████████╗███████╗████████╗██████╗ ██╗███████╗",
        "╚══██╔══╝██╔════╝╚══██╔══╝██╔══██╗██║██╔════╝",
        "   ██║   █████╗     ██║   ██████╔╝██║███████╗",
        "   ██║   ██╔══╝     ██║   ██╔══██╗██║╚════██║",
        "   ██║   ███████╗   ██║   ██║  ██║██║███████║",
        "   ╚═╝   ╚══════╝   ╚═╝   ╚═╝  ╚═╝╚═╝╚══════╝",
    ];

    let items = [
        "SOLO",
        "HOST LAN",
        "JOIN LAN",
        "JOIN RELAY",
        "SETTINGS",
        "QUIT",
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    for row in &ascii_art {
        lines.push(Line::from(Span::styled(
            *row,
            Style::default().fg(Color::Cyan),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    for (i, item) in items.iter().enumerate() {
        let marker = if i == selected { "▶ " } else { "  " };
        let style = if i == selected {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default().fg(Color::Gray)
        };
        lines.push(Line::from(Span::styled(format!("{marker}{item}"), style)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Arrow keys to select, Enter to confirm",
        Style::default().fg(Color::DarkGray),
    )));

    let para = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(para, area);
}

fn render_lobby_host(frame: &mut Frame, room_code: &str, players: &[String]) {
    let area = frame.area();
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("ROOM: {room_code}"),
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
    ];
    for p in players {
        lines.push(Line::from(Span::styled(
            p.as_str(),
            Style::default().fg(Color::White),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press R=Ready, T=Chat, Q=Quit",
        Style::default().fg(Color::DarkGray),
    )));
    let para = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(para, area);
}

fn render_lobby_client(frame: &mut Frame, room_code: &str, players: &[String]) {
    render_lobby_host(frame, room_code, players);
}

fn render_playing(
    frame: &mut Frame,
    engine: &tetris_core::engine::Engine<10, 20>,
    clear_flash: u8,
    score_flash: u8,
    prev_grid: &mut [[CellType; 10]; 20],
    prev_flash_mask: &mut u32,
    prev_half: &mut bool,
) {
    let area = frame.area();
    let use_half = area.height < 40;
    let side_w: u16 = 12;

    let layout = Layout::horizontal([
        Constraint::Length(side_w),
        Constraint::Fill(1),
        Constraint::Length(side_w),
    ])
    .split(area);

    render_hold(frame, engine, layout[0]);
    render_board(
        frame,
        engine,
        layout[1],
        use_half,
        clear_flash,
        prev_grid,
        prev_flash_mask,
        prev_half,
    );
    render_sidebar(frame, engine, layout[2], score_flash);
}

fn render_hold(frame: &mut Frame, engine: &tetris_core::engine::Engine<10, 20>, area: Rect) {
    let block = Block::default().title(" HOLD ").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if !engine.has_hold {
        return;
    }

    let piece = engine.state.hold;
    let shape = &PIECES[piece as usize].rot[0];
    let color = piece_color(piece);
    let mut lines = Vec::new();

    for y in 0..4 {
        let mut spans = Vec::new();
        for x in 0..4 {
            if shape.row[y] & (1 << x) != 0 {
                spans.push(Span::styled("██", Style::default().fg(color)));
            } else {
                spans.push(Span::raw("  "));
            }
        }
        lines.push(Line::from(spans));
    }
    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn render_board(
    frame: &mut Frame,
    engine: &tetris_core::engine::Engine<10, 20>,
    area: Rect,
    use_half: bool,
    clear_flash: u8,
    prev_grid: &mut [[CellType; 10]; 20],
    prev_flash_mask: &mut u32,
    prev_half: &mut bool,
) {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let st = &engine.state;
    let ghost_y = get_ghost_y(st);
    let active_shape = &PIECES[st.piece as usize].rot[st.rot as usize];

    let mut grid = [[CellType::Empty; 10]; 20];

    for y in 0..20 {
        for x in 0..10 {
            if st.board.rows[y] & (1u64 << x) != 0 {
                grid[y][x] = CellType::Locked;
            }
        }
    }

    if ghost_y >= 0 && !engine.game_over {
        for dy in 0..4 {
            for dx in 0..4 {
                if active_shape.row[dy] & (1 << dx) == 0 {
                    continue;
                }
                let gx = st.x as i32 + dx;
                let gy = ghost_y + dy as i32;
                if (0..10).contains(&gx)
                    && (0..20).contains(&gy)
                    && grid[gy as usize][gx as usize] == CellType::Empty
                {
                    grid[gy as usize][gx as usize] = CellType::Ghost;
                }
            }
        }
    }

    if !engine.game_over {
        for dy in 0..4 {
            for dx in 0..4 {
                if active_shape.row[dy] & (1 << dx) == 0 {
                    continue;
                }
                let px = st.x as i32 + dx;
                let py = st.y as i32 + dy as i32;
                if (0..10).contains(&px) && (0..20).contains(&py) {
                    grid[py as usize][px as usize] = CellType::Active(st.piece);
                }
            }
        }
    }

    let flash_mask = if clear_flash > 0 {
        st.last_clear_mask
    } else {
        0
    };

    if use_half {
        let mut lines = Vec::new();
        for y in (0..20).step_by(2) {
            let flash_top = flash_mask & (1 << y) != 0;
            let flash_bot = y + 1 < 20 && flash_mask & (1 << (y + 1)) != 0;
            let mut spans = Vec::new();
            for x in 0..10 {
                let top = grid[y][x];
                let bot = if y + 1 < 20 {
                    grid[y + 1][x]
                } else {
                    CellType::Empty
                };
                let (ch, style) = half_cell(top, bot, flash_top || flash_bot);
                spans.push(Span::styled(ch, style));
            }
            lines.push(Line::from(spans));
        }
        let para = Paragraph::new(lines);
        frame.render_widget(para, inner);
    } else {
        let mut lines = Vec::new();
        for y in 0..20 {
            let flashing = flash_mask & (1 << y) != 0;
            let mut spans = Vec::new();
            for x in 0..10 {
                let (text, style) = cell_style(grid[y][x], flashing);
                spans.push(Span::styled(text, style));
            }
            lines.push(Line::from(spans));
        }
        let para = Paragraph::new(lines);
        frame.render_widget(para, inner);
    }

    *prev_grid = grid;
    *prev_flash_mask = flash_mask;
    *prev_half = use_half;
}

use crate::app::CellType;

fn cell_style(cell: CellType, flashing: bool) -> (String, Style) {
    if flashing {
        return (
            "██".into(),
            Style::default().fg(Color::White).bg(Color::White),
        );
    }
    match cell {
        CellType::Empty => ("··".into(), Style::default().fg(Color::DarkGray)),
        CellType::Locked => ("██".into(), Style::default().fg(Color::Gray)),
        CellType::Ghost => ("░░".into(), Style::default().fg(Color::DarkGray)),
        CellType::Active(p) => ("██".into(), Style::default().fg(piece_color(p))),
    }
}

fn half_cell(top: CellType, bot: CellType, flashing: bool) -> (String, Style) {
    if flashing {
        return (
            "▀".into(),
            Style::default().fg(Color::White).bg(Color::White),
        );
    }
    let fg = cell_fg(top);
    let bg = cell_fg(bot);
    ("▀".into(), Style::default().fg(fg).bg(bg))
}

fn cell_fg(cell: CellType) -> Color {
    match cell {
        CellType::Empty => Color::Black,
        CellType::Locked => Color::Gray,
        CellType::Ghost => Color::DarkGray,
        CellType::Active(p) => piece_color(p),
    }
}

fn render_sidebar(
    frame: &mut Frame,
    engine: &tetris_core::engine::Engine<10, 20>,
    area: Rect,
    score_flash: u8,
) {
    let chunks = Layout::vertical([Constraint::Length(22), Constraint::Fill(1)]).split(area);

    let next_block = Block::default().title(" NEXT ").borders(Borders::ALL);
    let next_inner = next_block.inner(chunks[0]);
    frame.render_widget(next_block, chunks[0]);

    let mut next_lines = Vec::new();
    for i in 0..5 {
        let piece = engine.state.next[i];
        let shape = &PIECES[piece as usize].rot[0];
        let color = piece_color(piece);
        for y in 0..4 {
            let mut spans = Vec::new();
            for x in 0..4 {
                if shape.row[y] & (1 << x) != 0 {
                    spans.push(Span::styled("██", Style::default().fg(color)));
                } else {
                    spans.push(Span::raw("  "));
                }
            }
            next_lines.push(Line::from(spans));
        }
    }
    let next_para = Paragraph::new(next_lines);
    frame.render_widget(next_para, next_inner);

    let s = &engine.scorer;
    let score_color = if score_flash > 0 {
        Color::Yellow
    } else {
        Color::White
    };
    let stats_lines = vec![
        Line::from(""),
        Line::from(Span::styled("SCORE", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(
            format!("{}", s.score),
            Style::default().fg(score_color).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled("LEVEL", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(
            format!("{}", s.level),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled("LINES", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(
            format!("{}", s.total_lines),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled("COMBO", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(
            format!("{}", s.combo),
            Style::default().fg(Color::White),
        )),
    ];
    let stats_block = Block::default().borders(Borders::ALL);
    let stats_para = Paragraph::new(stats_lines).block(stats_block);
    frame.render_widget(stats_para, chunks[1]);
}

fn render_pause(frame: &mut Frame) {
    let area = frame.area();
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "PAUSED",
            Style::default().fg(Color::Yellow).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "P — Resume Game",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "Q — Quit to Menu",
            Style::default().fg(Color::Gray),
        )),
    ];
    let para = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(para, area);
}

fn render_game_over(
    frame: &mut Frame,
    score: u32,
    lines_cleared: u32,
    level: u32,
    max_combo: u32,
    tspin_count: u32,
) {
    let area = frame.area();
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "GAME OVER",
            Style::default().fg(Color::Red).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Score:     {score}"),
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            format!("Lines:     {lines_cleared}"),
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            format!("Level:     {level}"),
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            format!("Max Combo: {max_combo}"),
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            format!("T-Spins:   {tspin_count}"),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key for Menu",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let para = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(para, area);
}

#[allow(clippy::too_many_arguments)]
fn render_multi(
    frame: &mut Frame,
    engine: &tetris_core::engine::Engine<10, 20>,
    opponents: &[tetris_core::engine::Engine<10, 20>],
    opponent_names: &[String],
    clear_flash_timer: u8,
    score_flash_timer: u8,
    prev_grid: &mut [[crate::app::CellType; 10]; 20],
    prev_flash_mask: &mut u32,
    prev_half: &mut bool,
    _spectating: Option<usize>,
) {
    let area = frame.area();
    let chunks =
        Layout::horizontal([Constraint::Percentage(70), Constraint::Percentage(30)]).split(area);

    render_playing(frame, engine, clear_flash_timer, score_flash_timer, prev_grid, prev_flash_mask, prev_half);

    let right = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Fill(1),
    ])
    .split(chunks[1]);

    let title = Paragraph::new(vec![Line::from(Span::styled(
        "OPPONENTS",
        Style::default().fg(Color::Cyan).bold(),
    ))]);
    frame.render_widget(title, right[0]);

    for (idx, area) in right.iter().skip(1).take(3).enumerate() {
        if idx < opponents.len() {
            let name = opponent_names.get(idx).map_or("P?", String::as_str);
            render_opponent_panel(frame, &opponents[idx], name, *area);
        }
    }
}

fn render_opponent_panel(
    frame: &mut Frame,
    engine: &tetris_core::engine::Engine<10, 20>,
    name: &str,
    area: Rect,
) {
    let block = Block::default()
        .title(format!(" {name} "))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let st = &engine.state;
    let ghost_y = get_ghost_y(st);
    let shape = &PIECES[st.piece as usize].rot[st.rot as usize];
    let mut lines = Vec::new();

    for y in (0..20).step_by(2) {
        let mut spans = Vec::new();
        for x in 0..10 {
            let top = render_cell_for_state(st, shape, ghost_y, x, y);
            let bot = if y + 1 < 20 {
                render_cell_for_state(st, shape, ghost_y, x, y + 1)
            } else {
                crate::app::CellType::Empty
            };
            let (ch, style) = half_cell(top, bot, false);
            spans.push(Span::styled(ch, style));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_cell_for_state(
    st: &tetris_core::state::State<10, 20>,
    shape: &tetris_core::piece::Shape,
    ghost_y: i32,
    x: usize,
    y: usize,
) -> crate::app::CellType {
    if st.board.rows[y] & (1u64 << x) != 0 {
        return crate::app::CellType::Locked;
    }

    if ghost_y >= 0 {
        for dy in 0..4 {
            for dx in 0..4 {
                if shape.row[dy] & (1 << dx) == 0 {
                    continue;
                }
                let gx = st.x as i32 + dx;
                let gy = ghost_y + dy as i32;
                if gx == x as i32 && gy == y as i32 {
                    return crate::app::CellType::Ghost;
                }
            }
        }
    }

    for dy in 0..4 {
        for dx in 0..4 {
            if shape.row[dy] & (1 << dx) == 0 {
                continue;
            }
            let px = st.x as i32 + dx;
            let py = st.y as i32 + dy as i32;
            if px == x as i32 && py == y as i32 {
                return crate::app::CellType::Active(st.piece);
            }
        }
    }

    crate::app::CellType::Empty
}

fn render_game_over_multi(frame: &mut Frame, score: u32, lines: u32, level: u32, place: u8) {
    let area = frame.area();
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "GAME OVER",
            Style::default().fg(Color::Red).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Place: #{place}"),
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            format!("Score: {score}"),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            format!("Lines: {lines}"),
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            format!("Level: {level}"),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key for Menu",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let para = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(para, area);
}
