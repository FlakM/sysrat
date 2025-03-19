use std::borrow::BorrowMut;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Layout, Rect},
    style::{Color, Stylize},
    widgets::{Block, BorderType, StatefulWidget, Widget},
};

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Margin},
    style::{self, Modifier, Style},
    text::Text,
    widgets::{
        Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table,
    },
};
use style::palette::tailwind;
use unicode_width::UnicodeWidthStr;

use crate::app::App;

const INFO_TEXT: [&str; 1] =
    ["(Esc) quit | (↑) move up | (↓) move down | (←) move left | (→) move right"];

impl App {
    fn render_table(&self, area: Rect, buf: &mut Buffer) {
        let header_style = Style::default();
        let selected_row_style = Style::default().add_modifier(Modifier::REVERSED);
        let selected_col_style = Style::default();
        let selected_cell_style = Style::default().add_modifier(Modifier::REVERSED);

        let header = ["#", "timestamp", "pid", "ppid", "Command"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);
        let rows = self.processes.iter().enumerate().map(|(i, data)| {
            let item = std::iter::once(i.to_string())
                .chain(data.ref_array().iter().cloned())
                .collect::<Vec<_>>();

            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("{content}\n"))))
                .collect::<Row>()
                .style(Style::new())
                .height(1)
        });
        let bar = " █ ";
        let t = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(2),
                Constraint::Length(self.longest_item_lens.0 + 1),
                Constraint::Length(self.longest_item_lens.1 + 1),
                Constraint::Length(self.longest_item_lens.2 + 1),
                Constraint::Max(self.longest_item_lens.3),
            ],
        )
        .header(header)
        .row_highlight_style(selected_row_style)
        .column_highlight_style(selected_col_style)
        .cell_highlight_style(selected_cell_style)
        .highlight_symbol(Text::from(vec![
            "".into(),
            bar.into(),
            bar.into(),
            "".into(),
        ]))
        .highlight_spacing(HighlightSpacing::Always);
        let mut table_state = self.state.borrow_mut();

        StatefulWidget::render(t, area, buf, &mut table_state);
    }

    fn render_scrollbar(&self, area: Rect, buf: &mut Buffer) {
        StatefulWidget::render(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 0,
                horizontal: 0,
            }),
            buf,
            &mut self.scroll_state.borrow_mut(),
        )
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let info_footer = Paragraph::new(Text::from_iter(INFO_TEXT))
            .style(Style::new())
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new()),
            );
        Widget::render(info_footer, area, buf);
    }
}

impl Widget for &App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {

        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(4)]);
        let horizontal = Layout::horizontal([Constraint::Fill(1), Constraint::Max(3)]);


        let rects = vertical.split(area);

        let [table, scrollbar] = horizontal.areas(rects[0]);

        self.render_table(table, buf);
        self.render_scrollbar(scrollbar, buf);
        self.render_footer(rects[1], buf);
    }
}
