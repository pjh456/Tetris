use anyhow::Result;
use tetris_cli::app::AppState;
use tetris_cli::error::CliError;
use tetris_cli::game_loop::{AiOpponentConfig, run_game_loop};
use tetris_cli::multiplayer::{MultiplayerMode, validate_ai_opponent_weights};

fn main() -> Result<()> {
    let cfg = tetris_cli::config::load_config();
    let args = CliArgs::parse(std::env::args().skip(1))?;
    let state = args.initial_state();
    let ai_opponent = args.ai_opponent_config()?;
    if ai_opponent.is_some() {
        validate_ai_opponent_weights()?;
    }

    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(ratatui::restore));
        orig_hook(info);
    }));

    let mut terminal = ratatui::init();
    let mut draw_errors: Vec<String> = Vec::new();

    run_game_loop(
        state,
        &cfg,
        ai_opponent,
        tetris_cli::app::update,
        |st| match terminal.draw(|frame| tetris_cli::render::render(st, frame)) {
            Ok(_) => {}
            Err(e) => draw_errors.push(e.to_string()),
        },
    );

    ratatui::restore();
    for err in &draw_errors {
        eprintln!("terminal draw error: {err}");
    }
    Ok(())
}

const DEFAULT_AI_TEMP: f32 = 0.0;

#[derive(Debug, Clone, PartialEq)]
struct CliArgs {
    mode: Option<MultiplayerMode>,
    ai_opponents: usize,
    ai_temp: f32,
}

impl CliArgs {
    fn parse<I>(args: I) -> Result<Self, CliError>
    where
        I: IntoIterator<Item = String>,
    {
        let mut mode = None;
        let mut ai_opponents = 0usize;
        let mut ai_temp = DEFAULT_AI_TEMP;
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--ai-opponent" => ai_opponents = ai_opponents.saturating_add(1),
                "--ai-temp" => {
                    let value = next_arg(&mut args, "--ai-temp")?;
                    ai_temp = value.parse::<f32>().map_err(|e| {
                        CliError::Input(format!("invalid --ai-temp value {value}: {e}"))
                    })?;
                }
                "--host-p2p" => {
                    let value = next_arg(&mut args, "--host-p2p")?;
                    mode = Some(MultiplayerMode::host_p2p(&value)?);
                }
                "--join-p2p" => {
                    let value = next_arg(&mut args, "--join-p2p")?;
                    mode = Some(MultiplayerMode::join_p2p(&value)?);
                }
                "--join-relay" => {
                    let url = next_arg(&mut args, "--join-relay")?;
                    let room_code = next_arg(&mut args, "--join-relay")?;
                    mode = Some(MultiplayerMode::join_relay(url, room_code));
                }
                other => {
                    return Err(CliError::Input(format!("unknown argument {other}")));
                }
            }
        }

        Ok(Self {
            mode,
            ai_opponents,
            ai_temp,
        })
    }

    fn initial_state(&self) -> AppState {
        let Some(mode) = self.mode.clone() else {
            return AppState::Menu { selected: 0 };
        };
        let player_id = match mode {
            MultiplayerMode::HostP2p { .. } => 0,
            MultiplayerMode::JoinP2p { .. } | MultiplayerMode::JoinRelay { .. } => 1,
        };
        AppState::playing_multiplayer(mode, player_id)
    }

    fn ai_opponent_config(&self) -> Result<Option<AiOpponentConfig>, CliError> {
        if self.ai_opponents == 0 {
            return Ok(None);
        }
        if self.mode.is_none() {
            return Err(CliError::Input(
                "--ai-opponent requires --host-p2p, --join-p2p, or --join-relay".into(),
            ));
        }
        Ok(Some(AiOpponentConfig {
            count: self.ai_opponents,
            temperature: self.ai_temp,
        }))
    }
}

fn next_arg<I>(args: &mut I, flag: &str) -> Result<String, CliError>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| CliError::Input(format!("{flag} requires a value")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<CliArgs, CliError> {
        CliArgs::parse(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn ai_opponent_requires_multiplayer_mode() {
        let args = parse(&["--ai-opponent"]).unwrap();
        assert!(args.ai_opponent_config().is_err());
    }

    #[test]
    fn ai_opponent_accepts_temperature_with_host_mode() {
        let args = parse(&[
            "--host-p2p",
            "127.0.0.1:5000",
            "--ai-opponent",
            "--ai-temp",
            "0.5",
        ])
        .unwrap();
        let config = args.ai_opponent_config().unwrap().unwrap();
        assert_eq!(config.count, 1);
        assert_eq!(config.temperature, 0.5);
        assert!(matches!(
            args.initial_state(),
            AppState::PlayingMulti { session, .. }
                if matches!(session.mode, MultiplayerMode::HostP2p { .. })
        ));
    }

    #[test]
    fn repeated_ai_opponent_spawns_multiple_bots() {
        let args = parse(&[
            "--host-p2p",
            "127.0.0.1:5000",
            "--ai-opponent",
            "--ai-opponent",
        ])
        .unwrap();

        assert_eq!(args.ai_opponent_config().unwrap().unwrap().count, 2);
    }
}
