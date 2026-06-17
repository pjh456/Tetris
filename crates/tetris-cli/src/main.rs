mod app;
mod config;
mod error;
mod game_loop;
mod input;
mod render;

use anyhow::Result;

fn main() -> Result<()> {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(ratatui::restore));
        orig_hook(info);
    }));

    let mut terminal = ratatui::init();
    let cfg = config::load_config();
    let state = app::AppState::Menu { selected: 0 };
    let mut draw_errors: Vec<String> = Vec::new();

    game_loop::run_game_loop(state, &cfg, app::update, |st| {
        match terminal.draw(|frame| render::render(st, frame)) {
            Ok(_) => {}
            Err(e) => draw_errors.push(e.to_string()),
        }
    });

    ratatui::restore();
    for err in &draw_errors {
        eprintln!("terminal draw error: {err}");
    }
    Ok(())
}
