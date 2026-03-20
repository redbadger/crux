use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};

use crate::core::EventLog;

pub struct DebugPanel<'a> {
    event_log: &'a EventLog,
}

impl<'a> DebugPanel<'a> {
    pub const fn new(event_log: &'a EventLog) -> Self {
        Self { event_log }
    }
}

impl Widget for DebugPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title(Line::from(" Events & Effects ".bold()).centered())
            .border_set(border::THICK)
            .border_style(Style::new().fg(Color::DarkGray));

        let inner = block.inner(area);
        block.render(area, buf);

        let lines: Vec<Line> = self.event_log.lock().map_or_else(
            |_| vec![],
            |log| {
                let visible_height = inner.height as usize;
                log.iter()
                    .rev()
                    .take(visible_height)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .map(|entry| {
                        if entry.starts_with('→') {
                            Line::from(Span::styled(entry.clone(), Style::new().fg(Color::Cyan)))
                        } else {
                            Line::from(Span::styled(
                                entry.clone(),
                                Style::new().fg(Color::DarkGray),
                            ))
                        }
                    })
                    .collect()
            },
        );

        Paragraph::new(lines).render(inner, buf);
    }
}
