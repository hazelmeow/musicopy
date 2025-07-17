//! The app TUI.

pub mod log;

use crate::app::{App, AppMode, AppScreen};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Tabs, Widget, Wrap},
};
use tui_widgets::prompts::{Prompt, TextPrompt};

impl<'a> App<'a> {
    // we're using this instead of Widget::render because we also need the
    // frame to use TextPrompt
    pub fn render(&mut self, frame: &mut Frame) {
        use Constraint::{Length, Min};

        let [header_area, inner_area] = {
            let show_command = self.mode == AppMode::Command;

            let mut constraints = vec![
                // header area
                Length(1),
                // inner area
                Min(0),
            ];
            if show_command {
                constraints.push(Length(3))
            }

            let vertical = Layout::vertical(constraints);
            let areas = vertical.split(frame.area());

            let header_area = areas[0];
            let inner_area = areas[1];

            let mut next_area = 2;
            if show_command {
                let command_area = areas[next_area];
                next_area += 1;

                self.render_command(frame, command_area);
            }

            [header_area, inner_area]
        };

        let horizontal = Layout::horizontal([Min(0), Length(10), Length(12)]);
        let [tabs_area, id_area, title_area] = horizontal.areas(header_area);

        // tabs
        let selected_tab_index = match self.screen {
            AppScreen::Home => 0,
            AppScreen::Log => 1,
            AppScreen::Help => 2,
        };
        let titles = ["Home", "Log", "Help"]
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
                let key = if s == "Help" {
                    "?".blue().bold()
                } else {
                    (i + 1).to_string().blue().bold()
                };

                if i == selected_tab_index {
                    Line::from(vec!["[".blue().bold(), key, "] ".blue().bold(), s.into()])
                } else {
                    Line::from(vec!["<".blue().bold(), key, "> ".blue().bold(), s.into()])
                }
            })
            .collect::<Vec<_>>();
        Tabs::new(titles)
            .select(None)
            .padding("", "")
            .divider(" ")
            .render(tabs_area, frame.buffer_mut());

        // id
        if let Some(model) = &self.model {
            shorten_id(model.node.node_id.clone())
                .yellow()
                .render(id_area, frame.buffer_mut());
        }

        // title
        "Musicopy TUI".bold().render(title_area, frame.buffer_mut());

        match self.screen {
            AppScreen::Home => {
                self.render_home_screen(frame, inner_area);
            }
            AppScreen::Log => {
                self.render_log_screen(frame, inner_area);
            }
            AppScreen::Help => {
                self.render_help_screen(frame, inner_area);
            }
        }
    }

    fn render_command(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().border_set(border::THICK);

        TextPrompt::from("Command")
            .with_block(block)
            .draw(frame, area, &mut self.command_state);
    }

    fn render_home_screen(&mut self, frame: &mut Frame, area: Rect) {
        let title = Line::from(" Status ".bold());
        let instructions = Line::from(vec![
            " Command ".into(),
            "<:>".blue().bold(),
            " Quit ".into(),
            "<q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_top(instructions.right_aligned())
            .border_set(border::THICK);

        let Some(model) = &self.model else {
            let empty_text = Text::from("Waiting for model...");

            Paragraph::new(empty_text)
                .block(block)
                .render(area, frame.buffer_mut());
            return;
        };

        let active_servers = model
            .node
            .active_servers
            .iter()
            .map(|s| shorten_id(&s.node_id))
            .collect::<Vec<_>>()
            .join(", ");
        let pending_servers = model
            .node
            .pending_servers
            .iter()
            .map(|s| shorten_id(&s.node_id))
            .collect::<Vec<_>>()
            .join(", ");

        let active_clients = model
            .node
            .active_clients
            .iter()
            .map(|s| shorten_id(&s.node_id))
            .collect::<Vec<_>>()
            .join(", ");
        let pending_clients = model
            .node
            .pending_clients
            .iter()
            .map(|s| shorten_id(&s.node_id))
            .collect::<Vec<_>>()
            .join(", ");

        let mut lines = vec![
            Line::from("Network".bold()),
            Line::from(vec![
                "Node ID: ".into(),
                model.node.node_id.clone().yellow(),
            ]),
            Line::from(vec![
                "Home Relay: ".into(),
                model.node.home_relay.clone().yellow(),
            ]),
            Line::from(""),
            Line::from(vec!["Pending Servers: ".into(), pending_servers.yellow()]),
            Line::from(vec!["Active Servers: ".into(), active_servers.yellow()]),
            Line::from(""),
            Line::from(vec!["Pending Clients: ".into(), pending_clients.yellow()]),
            Line::from(vec!["Active Clients: ".into(), active_clients.yellow()]),
            Line::from(""),
            Line::from("Library".bold()),
        ];

        // library help text if empty
        if model.library.local_roots.is_empty() {
            lines.push(Line::from(vec![
                "Empty, add a path using ".into(),
                ":addlibrary <name> <path>".blue(),
            ]));
        }

        // library roots
        lines.extend(model.library.local_roots.iter().map(|root| {
            Line::from(vec![
                " - ".into(),
                root.name.clone().blue(),
                ": ".into(),
                root.path.clone().blue(),
                " (".green(),
                root.num_files.to_string().green(),
                ")".green(),
            ])
        }));

        let status_text = Text::from(lines);

        Paragraph::new(status_text)
            .block(block)
            .render(area, frame.buffer_mut());
    }

    fn render_help_screen(&mut self, frame: &mut Frame, area: Rect) {
        let title = Line::from(" Help ".bold());
        let instructions = Line::from(vec![
            " Command ".into(),
            "<:>".blue().bold(),
            " Quit ".into(),
            "<q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_top(instructions.right_aligned())
            .border_set(border::THICK);

        let lines = vec![
            Line::from("Navigation".bold()),
            Line::from(vec![
                " - ".into(),
                "<1>".blue(),
                " and ".into(),
                "<2>".blue(),
                " to change screens.".into(),
            ]),
            Line::from(vec![
                " - ".into(),
                "<:>".blue(),
                " to open the command prompt.".into(),
            ]),
            Line::from(vec![
                " - ".into(),
                "<q>".blue(),
                " or ".into(),
                "<ctrl + c>".blue(),
                " to quit.".into(),
            ]),
        ];

        Paragraph::new(lines)
            .block(block)
            .render(area, frame.buffer_mut());
    }
}

fn shorten_id(node_id: impl Into<String>) -> String {
    let mut node_id = node_id.into();
    node_id.truncate(6);
    node_id.push_str("..");
    node_id
}
