mod note;

use std::ops::Range;

use automerge::Change;
use crux_core::{render::Render, App};
use crux_macros::Effect;
use serde::{Deserialize, Serialize};

use crate::capabilities::pub_sub::PubSub;

pub use note::Note;

#[derive(Default)]
pub struct NoteEditor;

#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    Load(Vec<u8>),
    Insert(String),
    Replace(usize, usize, String),
    MoveCursor(usize),
    Select(usize, usize),
    Backspace,
    Delete,
    ReceiveChanges(Vec<u8>),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum TextCursor {
    Position(usize),
    Selection(Range<usize>),
}

impl Default for TextCursor {
    fn default() -> Self {
        TextCursor::Position(0)
    }
}

#[derive(Default)]
pub struct Model {
    note: Note,
    cursor: TextCursor,
}

// Same as Model for now, but may change
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ViewModel {
    text: String,
    cursor: TextCursor,
}

impl From<&Model> for ViewModel {
    fn from(model: &Model) -> Self {
        ViewModel {
            text: model.note.text(),
            cursor: model.cursor.clone(),
        }
    }
}

#[derive(Effect)]
#[effect(app = "NoteEditor")]
pub struct Capabilities {
    render: Render<Event>,
    pub_sub: PubSub<Event>,
}

impl App for NoteEditor {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;

    type Capabilities = Capabilities;

    fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
        match event {
            Event::Insert(text) => {
                let mut change = match &model.cursor {
                    TextCursor::Position(idx) => model.note.splice_text(*idx, 0, &text),
                    TextCursor::Selection(range) => {
                        model
                            .note
                            .splice_text(range.start, range.end - range.start, &text)
                    }
                };

                caps.pub_sub.publish(change.bytes().to_vec());

                let len = text.len();
                let idx = match &model.cursor {
                    TextCursor::Position(idx) => *idx,
                    TextCursor::Selection(range) => range.start,
                };
                model.cursor = TextCursor::Position(idx + len);
            }
            Event::Replace(from, to, text) => {
                let idx = from + text.len();
                model.cursor = TextCursor::Position(idx);

                let mut change = model.note.splice_text(from, to - from, &text);

                caps.pub_sub.publish(change.bytes().to_vec());
            }
            Event::MoveCursor(idx) => {
                model.cursor = TextCursor::Position(idx);
            }
            Event::Select(from, to) => {
                model.cursor = TextCursor::Selection(from..to);
            }
            Event::Backspace | Event::Delete => {
                let (new_index, mut change) = match &model.cursor {
                    TextCursor::Position(idx) => {
                        let idx = *idx;
                        let (remove, new_idx) = match event {
                            Event::Backspace => ((idx - 1)..idx, idx - 1),
                            Event::Delete => (idx..(idx + 1), idx),
                            _ => unreachable!(),
                        };

                        let change =
                            model
                                .note
                                .splice_text(remove.start, remove.end - remove.start, "");

                        (new_idx, change)
                    }
                    TextCursor::Selection(range) => {
                        let change =
                            model
                                .note
                                .splice_text(range.start, range.end - range.start, "");

                        (range.start, change)
                    }
                };

                model.cursor = TextCursor::Position(new_index);

                caps.pub_sub.publish(change.bytes().to_vec());
            }
            Event::Load(bytes) => {
                model.note = Note::load(&bytes);

                caps.pub_sub.subscribe(Event::ReceiveChanges);
            }
            Event::ReceiveChanges(bytes) => {
                let change = Change::from_bytes(bytes).expect("a valid change");

                model.note.apply_changes([change])
            }
        }

        caps.render.render();
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        model.into()
    }
}

#[cfg(test)]
mod editing_tests {
    use crux_core::{render::RenderOperation, testing::AppTester};

    use super::*;

    #[test]
    fn renders_text_and_cursor() {
        let app = AppTester::<NoteEditor, _>::default();

        let model = Model {
            note: Note::with_text("hello"),
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
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(3),
        };

        let update = app.update(Event::MoveCursor(5), &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        let view = app.view(&model);

        assert_eq!(view.text, "hello".to_string());
        assert_eq!(view.cursor, TextCursor::Position(5));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn changes_selection() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(3),
        };

        let update = app.update(Event::Select(2, 5), &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        let view = app.view(&model);

        assert_eq!(view.text, "hello".to_string());
        assert_eq!(view.cursor, TextCursor::Selection(2..5));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn inserts_text_at_cursor_and_renders() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(3),
        };

        let update = app.update(Event::Insert("l to the ".to_string()), &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        let view = app.view(&model);

        assert_eq!(view.text, "hell to the lo".to_string());
        assert_eq!(view.cursor, TextCursor::Position(12));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn replaces_selection_and_renders() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Selection(3..5),
        };

        let update = app.update(Event::Insert("ter skelter".to_string()), &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        let view = app.view(&model);

        assert_eq!(view.text, "helter skelter".to_string());
        assert_eq!(view.cursor, TextCursor::Position(14));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn replaces_range_and_renders() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(3),
        };

        let update = app.update(Event::Replace(1, 4, "i, y".to_string()), &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        let view = app.view(&model);

        assert_eq!(view.text, "hi, yo".to_string());
        assert_eq!(view.cursor, TextCursor::Position(5));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn replaces_empty_range_and_renders() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(3),
        };

        let update = app.update(
            Event::Replace(1, 1, "ey, just saying h".to_string()),
            &mut model,
        );
        let expected_effect = Effect::Render(RenderOperation);

        let view = app.view(&model);

        assert_eq!(view.text, "hey, just saying hello".to_string());
        assert_eq!(view.cursor, TextCursor::Position(18));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn removes_character_before_cursor() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(2),
        };

        let update = app.update(Event::Backspace, &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        let view = app.view(&model);

        assert_eq!(view.text, "hllo".to_string());
        assert_eq!(view.cursor, TextCursor::Position(1));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn removes_character_after_cursor() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(2),
        };

        let update = app.update(Event::Delete, &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        let view = app.view(&model);

        assert_eq!(view.text, "helo".to_string());
        assert_eq!(view.cursor, TextCursor::Position(2));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn removes_selection_on_delete() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Selection(2..4),
        };

        let update = app.update(Event::Delete, &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        let view = app.view(&model);

        assert_eq!(view.text, "heo".to_string());
        assert_eq!(view.cursor, TextCursor::Position(2));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }

    #[test]
    fn removes_selection_on_backspace() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Selection(2..4),
        };

        let update = app.update(Event::Backspace, &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        let view = app.view(&model);

        assert_eq!(view.text, "heo".to_string());
        assert_eq!(view.cursor, TextCursor::Position(2));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }
}

#[cfg(test)]
mod sync_tests {
    use automerge::{transaction::Transactable, Automerge, ObjType};
    use crux_core::testing::AppTester;

    use crate::capabilities::pub_sub::PubSubOperation;

    use super::*;

    #[test]
    fn a_single_change_sync() {
        // need a common starting point
        let mut doc = Automerge::new();

        doc.transact(|t| t.put_object(automerge::ROOT, "body", ObjType::Text))
            .expect("to create a document");

        let doc = doc.save();

        let alice = AppTester::<NoteEditor, _>::default();
        let mut alice_model = Default::default();

        let reqs = alice
            .update(Event::Load(doc.clone()), &mut alice_model)
            .effects;
        reqs.iter()
            .find(|r| matches!(r.as_ref(), Effect::PubSub(PubSubOperation::Subscribe)))
            .expect("Alice didn't subscribe for updates");

        let bob = AppTester::<NoteEditor, _>::default();
        let mut bob_model = Default::default();

        let reqs = bob.update(Event::Load(doc), &mut bob_model).effects;
        let bob_sub = reqs
            .iter()
            .find(|r| matches!(r.as_ref(), Effect::PubSub(PubSubOperation::Subscribe)))
            .expect("Bob didn't subscribe for updates");

        // Done wiring up

        let update = alice.update(Event::Insert("Hello".to_string()), &mut alice_model);
        update
            .effects
            .iter()
            .filter_map(|eff| match eff.as_ref() {
                Effect::PubSub(PubSubOperation::Publish(it)) => Some(it),
                _ => None,
            })
            .for_each(|bytes| {
                // Forward the published changes to Bob
                let evts = bob_sub.resolve(bytes).events;

                for evt in evts {
                    bob.update(evt, &mut bob_model);
                }
            });

        // Both should end up with the same doc

        let alice_view = alice.view(&alice_model);
        let bob_view = bob.view(&bob_model);

        assert_eq!(alice_view.text, bob_view.text);
    }
}
