mod core;
mod counter_widget;
mod debug_panel;
mod http;
mod sse;

use std::io;
use std::sync::atomic::Ordering;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::Widget,
};
use shared::{Event as AppEvent, ViewModel};

use crate::core::{EventLog, RenderFlag};
use crate::counter_widget::CounterWidget;
use crate::debug_panel::DebugPanel;

/// owns all state and drives the event loop
struct App {
    core: core::Core,
    render_flag: RenderFlag,
    event_log: EventLog,
    cached_view: ViewModel,
    selected: usize,
    debug_mode: bool,
    exit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            core: core::new(),
            render_flag: core::new_render_flag(),
            event_log: core::new_log(),
            cached_view: ViewModel::default(),
            selected: 0,
            debug_mode: false,
            exit: false,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        // Start watching for SSE updates (gives us the current value immediately)
        self.dispatch(AppEvent::StartWatch);

        while !self.exit {
            // Only call core.view() when the render flag indicates a change
            if self.render_flag.swap(false, Ordering::Acquire) {
                self.cached_view = self.core.view();
            }

            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        // Poll for terminal events with a short timeout so we also
        // pick up async-driven redraws promptly
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event);
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Esc => self.exit = true,
            KeyCode::Char('d') => self.debug_mode = !self.debug_mode,
            KeyCode::Left | KeyCode::Char('h') => self.select_prev(),
            KeyCode::Right | KeyCode::Char('l') => self.select_next(),
            KeyCode::Enter | KeyCode::Char(' ') => self.press_selected(),
            KeyCode::Char('+' | '=') => self.dispatch(AppEvent::Increment),
            KeyCode::Char('-') => self.dispatch(AppEvent::Decrement),
            _ => {}
        }
    }

    const fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    const fn select_next(&mut self) {
        if self.selected < counter_widget::NUM_BUTTONS - 1 {
            self.selected += 1;
        }
    }

    fn press_selected(&self) {
        if let Some(event) = counter_widget::button_event(self.selected) {
            self.dispatch(event);
        }
    }

    fn dispatch(&self, event: AppEvent) {
        core::update(&self.core, event, &self.render_flag, &self.event_log);
    }
}

/// Top-level compositor — splits the screen when debug mode is active and
/// delegates to the [`CounterWidget`] and [`DebugPanel`] widgets.
impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.debug_mode {
            let [main_area, debug_area] =
                Layout::vertical([Constraint::Fill(1), Constraint::Percentage(40)]).areas(area);

            CounterWidget::new(&self.cached_view, self.selected).render(main_area, buf);
            DebugPanel::new(&self.event_log).render(debug_area, buf);
        } else {
            CounterWidget::new(&self.cached_view, self.selected).render(area, buf);
        }
    }
}

fn main() -> io::Result<()> {
    // Create a multi-threaded tokio runtime for async HTTP/SSE tasks.
    // Entering the runtime lets `tokio::spawn` work from any thread.
    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let _guard = runtime.enter();

    ratatui::run(|terminal| App::new().run(terminal))
}
