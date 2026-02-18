use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Styled, Stylize},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};
use shared::{Event as AppEvent, ViewModel};

const BUTTONS: [(&str, AppEvent); 2] = [
    ("Increment", AppEvent::Increment),
    ("Decrement", AppEvent::Decrement),
];

pub const NUM_BUTTONS: usize = BUTTONS.len();

#[allow(clippy::cast_possible_truncation)]
const NUM_BUTTONS_U16: u16 = BUTTONS.len() as u16;

/// Returns the `AppEvent` for the button at the given index, if valid.
pub fn button_event(index: usize) -> Option<AppEvent> {
    BUTTONS.get(index).map(|(_, event)| event.clone())
}

pub struct CounterWidget<'a> {
    view: &'a ViewModel,
    selected: usize,
}

impl<'a> CounterWidget<'a> {
    pub const fn new(view: &'a ViewModel, selected: usize) -> Self {
        Self { view, selected }
    }
}

impl Widget for CounterWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Crux Counter Example ".bold());
        let hints = vec![
            " Select ".into(),
            "<←→>".blue().bold(),
            " Confirm ".into(),
            "<Enter>".blue().bold(),
            " Debug ".into(),
            "<D>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ];
        let instructions = Line::from(hints);

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let inner = block.inner(area);
        block.render(area, buf);

        // Split inner into: space for subtitle | main content (count+status+buttons) | bottom pad
        // count(3) + gap(1) + status(1) + gap(1) + buttons(3) = 9
        let [top_space, main_content, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(9),
            Constraint::Fill(1),
        ])
        .areas(inner);

        // -- Subtitle (vertically centered in the space above the counter) --
        let [_, subtitle_area, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(top_space);

        let sub_title = Line::from("Rust Core, Rust Shell (Ratatui)".bold());
        Paragraph::new(sub_title)
            .centered()
            .render(subtitle_area, buf);

        // -- Main content areas --
        let [count_area, _, status_area, _, buttons_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .areas(main_content);

        // -- Count display --
        let counter_text = Text::from(vec![Line::from(self.view.text.clone().yellow().bold())]);
        let count_block = Block::bordered().border_set(border::PLAIN);
        Paragraph::new(counter_text)
            .centered()
            .block(count_block)
            .render(count_area, buf);

        // -- Status indicator --
        let status = if self.view.confirmed {
            Line::from(Span::styled("● confirmed", Style::new().fg(Color::Green)))
        } else {
            Line::from(Span::styled("○ pending…", Style::new().fg(Color::DarkGray)))
        };
        Paragraph::new(status).centered().render(status_area, buf);

        // -- Buttons --
        ButtonBar::new(self.selected).render(buttons_area, buf);
    }
}

struct ButtonBar {
    selected: usize,
}

impl ButtonBar {
    const fn new(selected: usize) -> Self {
        Self { selected }
    }
}

impl Widget for ButtonBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let button_width: u16 = 14;
        let gap_width: u16 = 2;
        let total_width = button_width * NUM_BUTTONS_U16 + gap_width * (NUM_BUTTONS_U16 - 1);

        let [_, button_strip, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(total_width),
            Constraint::Fill(1),
        ])
        .areas(area);

        let constraints: Vec<Constraint> = BUTTONS
            .iter()
            .enumerate()
            .flat_map(|(i, _)| {
                if i < BUTTONS.len() - 1 {
                    vec![
                        Constraint::Length(button_width),
                        Constraint::Length(gap_width),
                    ]
                } else {
                    vec![Constraint::Length(button_width)]
                }
            })
            .collect();

        let cols = Layout::horizontal(constraints).split(button_strip);

        let colors = [Color::Green, Color::Yellow];

        for (i, (label, _)) in BUTTONS.iter().enumerate() {
            let col = cols[i * 2]; // even indices are buttons, odd are gaps
            let is_selected = i == self.selected;
            let color = colors[i];

            let (text_style, bdr_set) = if is_selected {
                (
                    Style::new().fg(Color::Black).bg(color).bold(),
                    border::THICK,
                )
            } else {
                (Style::new().fg(color), border::PLAIN)
            };

            let line = Line::from((*label).set_style(text_style));
            let btn_block = Block::bordered()
                .border_set(bdr_set)
                .border_style(text_style);
            Paragraph::new(line)
                .centered()
                .style(text_style)
                .block(btn_block)
                .render(col, buf);
        }
    }
}
