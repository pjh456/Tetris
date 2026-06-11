mod app;
mod config;
mod game_loop;
mod input;
mod render;

use anyhow::Result;

fn main() -> Result<()> {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        orig_hook(info);
    }));

    let mut terminal = ratatui::init();
    let cfg = config::load_config();
    let state = app::AppState::Menu { selected: 0 };

    game_loop::run_game_loop(
        state,
        &cfg,
        |st, msg| app::update(st, msg),
        |st| {
            terminal
                .draw(|frame| render::render(st, frame))
                .ok();
        },
    );

    ratatui::restore();
    Ok(())
}
