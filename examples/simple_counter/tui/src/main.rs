use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Styled, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use shared::{Core, Counter, Effect, Event as AppEvent};

const BUTTONS: [(&str, AppEvent); 3] = [
    ("Increment", AppEvent::Increment),
    ("Decrement", AppEvent::Decrement),
    ("Reset", AppEvent::Reset),
];

#[allow(clippy::cast_possible_truncation)]
const NUM_BUTTONS: u16 = BUTTONS.len() as u16;

struct App {
    core: Core<Counter>,
    selected: usize,
    exit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            core: Core::new(),
            selected: 0,
            exit: false,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Esc => self.exit = true,
            KeyCode::Left | KeyCode::Char('h') => self.select_prev(),
            KeyCode::Right | KeyCode::Char('l') => self.select_next(),
            KeyCode::Enter | KeyCode::Char(' ') => self.press_selected(),
            KeyCode::Char('+' | '=') => self.dispatch(AppEvent::Increment),
            KeyCode::Char('-') => self.dispatch(AppEvent::Decrement),
            KeyCode::Char('0') => self.dispatch(AppEvent::Reset),
            _ => {}
        }
    }

    fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn select_next(&mut self) {
        if self.selected < BUTTONS.len() - 1 {
            self.selected += 1;
        }
    }

    fn press_selected(&self) {
        let (_, ref event) = BUTTONS[self.selected];
        self.dispatch(event.clone());
    }

    fn dispatch(&self, event: AppEvent) {
        for effect in self.core.process_event(event) {
            match effect {
                Effect::Render(_) => {
                    // The shell re-renders on the next loop iteration
                }
            }
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let view = self.core.view();

        let title = Line::from(" Simple Counter ".bold());
        let instructions = Line::from(vec![
            " Select ".into(),
            "<←→>".blue().bold(),
            " Confirm ".into(),
            "<Enter>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let inner = block.inner(area);
        block.render(area, buf);

        // Center content vertically: count display + gap + buttons = 7 rows
        let [_, content, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(7),
            Constraint::Fill(1),
        ])
        .areas(inner);

        let [count_area, _, buttons_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .areas(content);

        // -- Count display --
        let counter_text = Text::from(vec![Line::from(view.count.clone().yellow().bold())]);
        let count_block = Block::bordered().border_set(border::PLAIN);
        Paragraph::new(counter_text)
            .centered()
            .block(count_block)
            .render(count_area, buf);

        // -- Buttons --
        let button_width: u16 = 14;
        let gap_width: u16 = 2;
        let total_width = button_width * NUM_BUTTONS + gap_width * (NUM_BUTTONS - 1);

        let [_, button_strip, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(total_width),
            Constraint::Fill(1),
        ])
        .areas(buttons_area);

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

        let colors = [Color::Green, Color::Yellow, Color::Red];

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

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}
