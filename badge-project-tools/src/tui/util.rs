use derive_setters::Setters;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{Span, Stylize};
use ratatui::style::Style;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

#[derive(Debug, Default, Setters)]
pub struct Popup<'a> {
    #[setters(into)]
    title: Line<'a>,
    #[setters(into)]
    content: Text<'a>,
    border_style: Style,
    title_style: Style,
    style: Style,
}

impl Widget for Popup<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // ensure that all cells under the popup are cleared to avoid leaking content
        Clear.render(area, buf);
        let block = Block::new()
            .title(self.title)
            .title_style(self.title_style)
            .borders(Borders::ALL)
            .border_style(self.border_style);
        Paragraph::new(self.content)
            .wrap(Wrap { trim: true })
            .style(self.style)
            .block(block)
            .render(area, buf);
    }
}

pub type ControlsInfo = &'static [(&'static str, &'static str)];

pub fn make_controls_line(controls: ControlsInfo) -> Line<'static> {
    let mut controls = controls.iter()
        .flat_map(|(key, action)| [
            Span::styled("<", Style::new().dark_gray()),
            Span::styled(*key, Style::new().yellow()),
            Span::styled("> ", Style::new().dark_gray()),
            Span::raw(format!("to {action}")),
            Span::styled(" | ", Style::new().dark_gray())
        ])
        .collect::<Vec<_>>();
    controls.pop();
    Line::from(controls)
}