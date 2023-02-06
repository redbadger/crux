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
    use std::collections::VecDeque;

    use crux_core::testing::{AppTester, TestEffect};

    use crate::capabilities::pub_sub::PubSubOperation;

    use super::*;

    struct Peer {
        app: AppTester<NoteEditor, Effect>,
        model: Model,
        subscription: Option<TestEffect<Effect, Event>>,
        edits: VecDeque<Vec<u8>>,
    }

    // A jig to make testing sync a bit easier

    impl Peer {
        fn new() -> Self {
            let app = AppTester::<_, _>::default();
            let model = Default::default();

            Self {
                app,
                model,
                subscription: None,
                edits: VecDeque::new(),
            }
        }

        // Update, picking out and keeping PubSub effects
        fn update(&mut self, event: Event) -> (Vec<Effect>, Vec<Event>) {
            let update = self.app.update(event, &mut self.model);

            let events = update.events;
            let mut effects = Vec::new();

            for effect in update.effects {
                match effect.as_ref() {
                    Effect::PubSub(PubSubOperation::Subscribe) => {
                        self.subscription = Some(effect);
                    }
                    Effect::PubSub(PubSubOperation::Publish(bytes)) => {
                        self.edits.push_back(bytes.clone());
                    }
                    ef => effects.push(ef.clone()),
                }
            }

            (effects, events)
        }

        fn view(&self) -> ViewModel {
            self.app.view(&self.model)
        }

        fn send_edits(&mut self, edits: &[Vec<u8>]) -> (Vec<Effect>, Vec<Event>) {
            let subscription = self.subscription.as_ref().expect("to have a subscription");

            let mut effects = Vec::new();
            let mut events = Vec::new();

            let evs = edits
                .iter()
                .flat_map(|ed| subscription.resolve(ed).events)
                .collect::<Vec<_>>();

            for event in evs {
                let (mut eff, mut ev) = self.update(event);

                effects.append(&mut eff);
                events.append(&mut ev);
            }

            (effects, events)
        }
    }

    fn make_alice_and_bob() -> (Peer, Peer) {
        let note = Note::new().save();

        let mut alice = Peer::new();
        let mut bob = Peer::new();

        alice.update(Event::Load(note.clone()));
        bob.update(Event::Load(note));

        (alice, bob)
    }

    #[test]
    fn a_single_change_sync() {
        let (mut alice, mut bob) = make_alice_and_bob();

        alice.update(Event::Insert("Hello".to_string()));
        let edits = alice.edits.drain(0..).collect::<Vec<_>>();

        bob.send_edits(edits.as_ref());

        let alice_view = alice.view();
        let bob_view = bob.view();

        assert_eq!(alice_view.text, bob_view.text);
    }
}
