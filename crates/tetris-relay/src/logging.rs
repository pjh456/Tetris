use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::Arc;

use tracing::{Event, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::{FormatTime, SystemTime};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

/// Plain text formatter producing fixed lines: `<rfc3339> [LEVEL]: message`.
/// No JSON, no target, no span — readable for a headless service log.
struct PlainFormat;

impl<S, N> FormatEvent<S, N> for PlainFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        SystemTime.format_time(&mut writer)?;
        write!(writer, " [{}]: ", event.metadata().level())?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

/// Initialize tracing. Level is controlled by `RUST_LOG` (default `info`).
/// When `log_file` is given, lines are appended to that file; otherwise they
/// go to the terminal.
pub fn init_logging(log_file: Option<PathBuf>) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt()
        .event_format(PlainFormat)
        .with_env_filter(filter);

    match log_file {
        Some(path) => match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(file) => builder.with_writer(Arc::new(file)).init(),
            Err(e) => {
                builder.init();
                tracing::error!("failed to open log file {}: {e}", path.display());
            }
        },
        None => builder.init(),
    }
}
