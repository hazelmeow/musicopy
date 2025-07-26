use crate::{
    event::{Event, EventHandler, app_send},
    ui::log::LogState,
};
use anyhow::Context;
use musicopy::{Core, CoreOptions, Model};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};
use std::sync::Arc;
use tui_widgets::prompts::{State, Status, TextState};

/// Application.
#[derive(Debug)]
pub struct App<'a> {
    pub running: bool,
    pub events: EventHandler,

    pub core: Arc<Core>,

    pub mode: AppMode,
    pub screen: AppScreen,

    pub messages: Vec<String>,

    pub log_state: LogState,
    pub command_state: TextState<'a>,
    pub model: Option<Model>, // TODO: would be nice to make this not an option
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppScreen {
    #[default]
    Home,
    Log,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    Default,
    Command,
}

/// Application events.
#[derive(Debug)]
pub enum AppEvent {
    Log(String),

    Exit,

    CommandMode,
    ExitMode,

    Screen(AppScreen),

    Model(Box<Model>),
}

macro_rules! app_log {
    ($($arg:tt)*) => {
        let _ = crate::event::app_send!(crate::app::AppEvent::Log(format!($($arg)*)));
    };
}
pub(crate) use app_log;

impl<'a> App<'a> {
    /// Constructs a new instance of [`App`].
    pub async fn new(in_memory: bool) -> anyhow::Result<Self> {
        // initialize as early as possible
        let events = EventHandler::new();

        let core = Core::new(
            Arc::new(AppEventHandler),
            CoreOptions {
                init_logging: false,
                in_memory,
            },
        )?;

        Ok(Self {
            running: true,
            events,

            core,

            mode: AppMode::default(),
            screen: AppScreen::default(),

            messages: Vec::new(),

            log_state: LogState::default(),
            command_state: TextState::default(),
            model: None,
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => {
                    if let crossterm::event::Event::Key(key_event) = event {
                        self.handle_key_events(key_event)?
                    }
                }
                Event::App(app_event) => self
                    .handle_app_events(app_event)
                    .context("handling app event failed")?,
            }
        }

        // shut down core
        self.core.shutdown()?;

        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> anyhow::Result<()> {
        match self.mode {
            AppMode::Default => match (self.screen, key_event.code) {
                // : or / to enter command mode
                (_, KeyCode::Char(':') | KeyCode::Char('/')) => {
                    self.events.send(AppEvent::CommandMode)
                }

                // change screens
                (_, KeyCode::Char('1')) => self.events.send(AppEvent::Screen(AppScreen::Home)),
                (_, KeyCode::Char('2')) => self.events.send(AppEvent::Screen(AppScreen::Log)),
                (_, KeyCode::Char('?')) => self.events.send(AppEvent::Screen(AppScreen::Help)),

                // esc or q to quit
                (_, KeyCode::Esc | KeyCode::Char('q')) => self.events.send(AppEvent::Exit),
                // ctrl+c to quit
                (_, KeyCode::Char('c' | 'C')) if key_event.modifiers == KeyModifiers::CONTROL => {
                    self.events.send(AppEvent::Exit)
                }

                // log screen
                (AppScreen::Log, KeyCode::Up) => {
                    self.log_state.scroll_up();
                }
                (AppScreen::Log, KeyCode::Down) => {
                    self.log_state.scroll_down();
                }
                (AppScreen::Log, KeyCode::PageUp) => {
                    self.log_state.scroll_page_up();
                }
                (AppScreen::Log, KeyCode::PageDown) => {
                    self.log_state.scroll_page_down();
                }
                (AppScreen::Log, KeyCode::Home | KeyCode::Char('g')) => {
                    self.log_state.scroll_to_top();
                }
                (AppScreen::Log, KeyCode::End | KeyCode::Char('G')) => {
                    self.log_state.scroll_to_bottom();
                }
                (AppScreen::Log, KeyCode::Char('f')) => {
                    self.log_state.toggle_tail();
                }

                _ => {}
            },

            AppMode::Command => {
                self.command_state.handle_key_event(key_event);

                match self.command_state.status() {
                    Status::Done => {
                        let command = self.command_state.value().to_string();

                        if let Err(e) = self.handle_command(command) {
                            app_log!("Error: {e:#}");
                        }

                        self.events.send(AppEvent::ExitMode);
                    }
                    Status::Aborted => self.events.send(AppEvent::ExitMode),
                    Status::Pending => {}
                }
            }
        }

        Ok(())
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    pub fn handle_app_events(&mut self, app_event: AppEvent) -> anyhow::Result<()> {
        match app_event {
            AppEvent::Log(s) => self.messages.push(s),

            AppEvent::Exit => self.exit(),

            AppEvent::CommandMode => {
                self.mode = AppMode::Command;
                self.command_state.focus();
            }
            AppEvent::ExitMode => {
                self.mode = AppMode::Default;
                self.command_state = TextState::default();
            }

            AppEvent::Screen(screen) => {
                self.screen = screen;
            }

            AppEvent::Model(model) => {
                self.model = Some(*model);
            }
        }
        Ok(())
    }

    pub fn handle_command(&mut self, command: String) -> anyhow::Result<()> {
        let parts = command.split_whitespace().collect::<Vec<_>>();

        if parts.is_empty() {
            return Ok(());
        }

        match parts[0] {
            "q" => self.events.send(AppEvent::Exit),

            "addlibrary" => {
                if parts.len() < 3 {
                    anyhow::bail!("usage: addlibrary <name> <path>");
                }

                let name = parts[1].to_string();
                let path = parts[2].to_string();
                self.core.add_library_root(name, path)?;
            }

            "removelibrary" => {
                if parts.len() < 2 {
                    anyhow::bail!("usage: removelibrary <name>");
                }

                let name = parts[1].to_string();
                self.core.remove_library_root(name)?;
            }

            "resetdb" => {
                self.core.reset_database()?;
                self.core.rescan_library()?;
            }

            "a" | "accept" => {
                app_log!("accepting pending servers");

                let Some(model) = &self.model else {
                    anyhow::bail!("model not initialized");
                };

                for server in &model.node.servers {
                    if !server.accepted {
                        app_log!("accepting server: {}", server.node_id);
                        self.core.accept_connection(&server.node_id)?;
                    }
                }
            }

            "c" | "connect" => {
                if parts.len() < 2 {
                    anyhow::bail!("usage: connect <node_id>");
                }

                let node_id = parts[1].to_string();

                app_log!("connecting to node: {}", node_id);

                let core = self.core.clone();
                tokio::spawn(async move {
                    if let Err(e) = core.connect(&node_id).await {
                        app_log!("error connecting to node {}: {e:#}", node_id);
                    }
                });
            }

            "dl" | "download" => {
                if parts.len() < 2 {
                    anyhow::bail!("usage: download <client #>");
                }

                let client_num = parts[1]
                    .parse::<usize>()
                    .context("failed to parse client number")?;

                if client_num == 0 {
                    anyhow::bail!("client number must be greater than 0");
                }

                let Some(model) = &self.model else {
                    anyhow::bail!("model not initialized");
                };
                let node_id = model
                    .node
                    .clients
                    .iter()
                    .filter(|c| c.accepted)
                    .nth(client_num - 1)
                    .ok_or_else(|| anyhow::anyhow!("client number out of range"))?
                    .node_id
                    .clone();

                app_log!("downloading from client: {}", client_num);

                let core = self.core.clone();
                tokio::spawn(async move {
                    if let Err(e) = core.download_all(&node_id, "/tmp/musicopy-dl") {
                        app_log!("error downloading from client {}: {e:#}", client_num);
                    }
                });
            }

            "help" | "h" | "?" => {
                app_send!(AppEvent::Screen(AppScreen::Help));
            }

            _ => {
                anyhow::bail!("unknown command: {command}");
            }
        }
        Ok(())
    }

    /// Exit the app.
    fn exit(&mut self) {
        self.running = false;
    }
}

struct AppEventHandler;

impl musicopy::EventHandler for AppEventHandler {
    fn on_update(&self, model: Model) {
        app_send!(AppEvent::Model(Box::new(model)));
    }
}
