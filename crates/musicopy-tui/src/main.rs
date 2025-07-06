mod app;
mod event;
mod ui;

use crate::{
    app::{App, AppEvent},
    event::app_send,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initialize app
    let app = App::new().await?;

    // set up global logger
    let logger = AppLogger::new();
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(log::LevelFilter::Debug);

    // run tui
    let terminal = ratatui::init();
    let app_result = app.run(terminal).await;
    ratatui::restore();
    app_result
}

/// Logger implementation that logs to the TUI.
struct AppLogger {
    filter: env_filter::Filter,
}

impl AppLogger {
    fn new() -> Self {
        let mut filter_builder = env_filter::Builder::new();
        if let Ok(filter) = &std::env::var("RUST_LOG") {
            filter_builder.parse(filter);
        }
        Self {
            filter: filter_builder.build(),
        }
    }
}

impl log::Log for AppLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.filter.matches(record) {
            let s = record.args().to_string();
            app_send!(AppEvent::Log(s));
        }
    }

    fn flush(&self) {}
}
