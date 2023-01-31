use std::ops::Range;

use crux_core::{render::Render, App};
use crux_macros::Effect;
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct NoteEditor;

#[derive(Serialize, Deserialize)]
pub enum Event {
    Insert(String),
    MoveCursor(usize),
    Select(usize, usize),
    Backspace,
    Delete,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
enum TextCursor {
    Position(usize),
    Selection(Range<usize>),
}

impl Default for TextCursor {
    fn default() -> Self {
        TextCursor::Position(0)
    }
}

#[derive(Default)]
struct Model {
    text: String,
    cursor: TextCursor,
}

// Same as Model for now, but may change
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct ViewModel {
    text: String,
    cursor: TextCursor,
}

impl From<&Model> for ViewModel {
    fn from(model: &Model) -> Self {
        ViewModel {
            text: model.text.clone(),
            cursor: model.cursor.clone(),
        }
    }
}

#[derive(Effect)]
#[effect(app = "NoteEditor")]
struct Capabilities {
    render: Render<Event>,
}

impl App for NoteEditor {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;

    type Capabilities = Capabilities;

    fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
        match event {
            Event::Insert(text) => {
                let len = text.len();

                let idx = match &model.cursor {
                    TextCursor::Position(idx) => *idx,
                    TextCursor::Selection(range) => {
                        model.text.replace_range(range.clone(), "");

                        range.start
                    }
                };

                model.text.insert_str(idx, &text);
                model.cursor = TextCursor::Position(idx + len);
            }
            Event::MoveCursor(idx) => {
                model.cursor = TextCursor::Position(idx);
            }
            Event::Select(from, to) => {
                model.cursor = TextCursor::Selection(from..to);
            }
            Event::Backspace | Event::Delete => {
                let new_index = match &model.cursor {
                    TextCursor::Position(idx) => {
                        let idx = *idx;
                        let (remove, new_idx) = match event {
                            Event::Backspace => ((idx - 1)..idx, idx - 1),
                            Event::Delete => (idx..(idx + 1), idx),
                            _ => unreachable!(),
                        };

                        model.text.replace_range(remove, "");

                        new_idx
                    }
                    TextCursor::Selection(range) => {
                        model.text.replace_range(range.clone(), "");

                        range.start
                    }
                };

                model.cursor = TextCursor::Position(new_index);
            }
        }

        caps.render.render();
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        model.into()
    }
}

#[cfg(test)]
mod tests {
    use crux_core::{render::RenderOperation, testing::AppTester};

    use super::*;

    #[test]
    fn renders_text_and_cursor() {
        let app = AppTester::<NoteEditor, _>::default();

        let model = Model {
            text: "hello".to_string(),
            cursor: TextCursor::Position(2),
        };
        let actual = app.view(&model);

        let expected = ViewModel {
            text: "hello".to_string(),
            cursor: TextCursor::Position(2),
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn moves_cursor() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            text: "hello".to_string(),
            cursor: TextCursor::Position(3),
        };

        let update = app.update(Event::MoveCursor(5), &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        assert_eq!(model.text, "hello".to_string());
        assert_eq!(model.cursor, TextCursor::Position(5));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn changes_selection() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            text: "hello".to_string(),
            cursor: TextCursor::Position(3),
        };

        let update = app.update(Event::Select(2, 5), &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        assert_eq!(model.text, "hello".to_string());
        assert_eq!(model.cursor, TextCursor::Selection(2..5));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn inserts_text_at_cursor_and_renders() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            text: "hello".to_string(),
            cursor: TextCursor::Position(3),
        };

        let update = app.update(Event::Insert("l to the ".to_string()), &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        assert_eq!(model.text, "hell to the lo".to_string());
        assert_eq!(model.cursor, TextCursor::Position(12));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn replaces_selection_and_renders() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            text: "hello".to_string(),
            cursor: TextCursor::Selection(3..5),
        };

        let update = app.update(Event::Insert("ter skelter".to_string()), &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        assert_eq!(model.text, "helter skelter".to_string());
        assert_eq!(model.cursor, TextCursor::Position(14));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn removes_character_before_cursor() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            text: "hello".to_string(),
            cursor: TextCursor::Position(2),
        };

        let update = app.update(Event::Backspace, &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        assert_eq!(model.text, "hllo".to_string());
        assert_eq!(model.cursor, TextCursor::Position(1));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn removes_character_after_cursor() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            text: "hello".to_string(),
            cursor: TextCursor::Position(2),
        };

        let update = app.update(Event::Delete, &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        assert_eq!(model.text, "helo".to_string());
        assert_eq!(model.cursor, TextCursor::Position(2));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn removes_selection_on_delete() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            text: "hello".to_string(),
            cursor: TextCursor::Selection(2..4),
        };

        let update = app.update(Event::Delete, &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        assert_eq!(model.text, "heo".to_string());
        assert_eq!(model.cursor, TextCursor::Position(2));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn removes_selection_on_backspace() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            text: "hello".to_string(),
            cursor: TextCursor::Selection(2..4),
        };

        let update = app.update(Event::Backspace, &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        assert_eq!(model.text, "heo".to_string());
        assert_eq!(model.cursor, TextCursor::Position(2));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }
}
