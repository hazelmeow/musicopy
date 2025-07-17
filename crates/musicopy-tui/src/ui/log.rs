use crate::app::App;
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Scrollbar, ScrollbarOrientation, ScrollbarState, Widget},
};

#[derive(Debug)]
pub struct LogState {
    vertical_scroll: usize,
    tail: bool,

    scrollbar_state: ScrollbarState,
    content_height: usize,
    viewport_height: usize,
}

impl Default for LogState {
    fn default() -> Self {
        LogState {
            vertical_scroll: 0,
            tail: true,

            scrollbar_state: ScrollbarState::default(),
            content_height: 0,
            viewport_height: 0,
        }
    }
}

impl LogState {
    pub fn scroll_up(&mut self) {
        if self.tail {
            // stop tailing but don't scroll
            self.tail = false;
        } else {
            self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
        }
    }

    pub fn scroll_down(&mut self) {
        if self.is_at_bottom() {
            // start tailing but don't scroll
            self.tail = true;
        } else {
            self.vertical_scroll = self.vertical_scroll.saturating_add(1);
        }
    }

    pub fn scroll_page_up(&mut self) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(self.viewport_height);
        self.tail = false;
    }

    pub fn scroll_page_down(&mut self) {
        self.vertical_scroll = self.vertical_scroll.saturating_add(self.viewport_height);

        if self.is_at_bottom() {
            self.tail = true;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.vertical_scroll = 0;
        self.tail = false;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.vertical_scroll = self.content_height.saturating_sub(self.viewport_height);
        self.tail = true;
    }

    pub fn toggle_tail(&mut self) {
        if self.tail {
            self.tail = false;
        } else {
            self.scroll_to_bottom();
        }
    }

    pub fn is_at_bottom(&self) -> bool {
        self.vertical_scroll >= self.content_height.saturating_sub(self.viewport_height)
    }
}

impl<'a> App<'a> {
    /// - Draw lines
    /// - Draw borders
    /// - Draw scrollbar
    pub(super) fn render_log_screen(&mut self, frame: &mut Frame, screen_area: Rect) {
        let block = {
            let title = Line::from(" Log ".bold());
            let top_instructions = Line::from(vec![
                " Command ".into(),
                "<:>".blue().bold(),
                " Quit ".into(),
                "<q> ".blue().bold(),
            ]);

            let block = Block::bordered()
                .border_set(border::THICK)
                .title(title.centered())
                .title_top(top_instructions.right_aligned());

            // bottom right instructions
            let bottom_instructions = if self.log_state.tail {
                Line::from(vec![" Stop Following ".into(), "<f> ".blue().bold()])
            } else {
                Line::from(vec![" Follow ".into(), "<f> ".blue().bold()])
            };
            let block = block.title_bottom(bottom_instructions.right_aligned());

            // bottom left indicator
            let bottom_indicator = if self.log_state.is_at_bottom() {
                if self.log_state.tail {
                    Line::from(" … following ".blue().bold())
                } else {
                    Line::from(" … at end ".blue().bold())
                }
            } else {
                Line::from(" ↓ more below ".blue().bold())
            };
            block.title_bottom(bottom_indicator.left_aligned())
        };

        let inner_area = block.inner(screen_area);

        // update viewport height
        self.log_state.viewport_height = inner_area.height as usize;

        // build entries from messages
        // TODO: more formatting w/ metadata, line wrapping
        let entries = self
            .messages
            .iter()
            .map(|s| vec![Line::from(s.as_str())])
            .collect::<Vec<_>>();

        // count total height of all entries
        let content_height = {
            let mut content_height = 0;
            for text in &entries {
                content_height += text.len();
            }
            content_height
        };
        self.log_state.content_height = content_height;

        // if tail is enabled, scroll to the bottom
        if self.log_state.tail {
            self.log_state.vertical_scroll =
                content_height.saturating_sub(inner_area.height as usize);
        }

        // calculate number of lines to skip
        let mut scroll_skip = self.log_state.vertical_scroll;

        // draw lines to buffer
        let mut screen_y = 0;
        'outer: for entry in entries {
            for line in &entry {
                if scroll_skip > 0 {
                    scroll_skip -= 1;
                    continue;
                }

                frame.buffer_mut().set_line(
                    inner_area.left(),
                    inner_area.top() + screen_y,
                    line,
                    u16::MAX,
                );

                screen_y += 1;

                if screen_y >= inner_area.height {
                    break 'outer;
                }
            }
        }

        // draw borders
        block.render(screen_area, frame.buffer_mut());

        // update scrollbar state
        self.log_state.scrollbar_state = self
            .log_state
            .scrollbar_state
            .content_length(content_height.saturating_sub(inner_area.height as usize))
            .viewport_content_length(content_height.saturating_sub(inner_area.height as usize))
            .position(self.log_state.vertical_scroll);

        // draw scrollbar
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(ratatui::symbols::scrollbar::VERTICAL)
            .begin_symbol(None)
            .track_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(
            scrollbar,
            screen_area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut self.log_state.scrollbar_state,
        );
    }
}
