mod note;

use std::ops::Range;

use automerge::Change;
use crux_core::{render::Render, App};
use crux_kv::{KeyValue, KeyValueOutput};
use crux_macros::Effect;
use serde::{Deserialize, Serialize};

use crate::capabilities::{
    pub_sub::PubSub,
    timer::{Timer, TimerOutput},
};

pub use note::Note;

use self::note::EditObserver;

#[derive(Default)]
pub struct NoteEditor;

#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    Open,
    Insert(String),
    Replace(usize, usize, String),
    MoveCursor(usize),
    Select(usize, usize),
    Backspace,
    Delete,
    ReceiveChanges(Vec<u8>),
    EditTimer(TimerOutput),
    Written(KeyValueOutput),
    Load(KeyValueOutput),
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
struct EditTimer {
    current_id: Option<u64>,
    next_id: u64,
}

impl EditTimer {
    fn start(&mut self, timer: &Timer<Event>) {
        if let Some(id) = self.current_id {
            println!("Cancelling timer {id}");
            timer.cancel(id);
        }
        self.current_id = None;

        println!("Starting timer {}", self.next_id);
        timer.start(self.next_id, EDIT_TIMER, Event::EditTimer);
    }

    fn was_created(&mut self, id: u64) {
        println!("Timer {id} created, setting next_id to {}", id + 1);
        self.next_id = id + 1;
        self.current_id = Some(id);
    }

    fn finished(&mut self, id: u64) {
        println!("Timer {id} finished");
        self.current_id = None;
    }
}

#[derive(Default)]
pub struct Model {
    note: Note,
    cursor: TextCursor,
    edit_timer: EditTimer,
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
    timer: Timer<Event>,
    render: Render<Event>,
    pub_sub: PubSub<Event>,
    key_value: KeyValue<Event>,
}

const EDIT_TIMER: usize = 1000;

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
                model.edit_timer.start(&caps.timer);

                let len = text.chars().count();
                let idx = match &model.cursor {
                    TextCursor::Position(idx) => *idx,
                    TextCursor::Selection(range) => range.start,
                };
                model.cursor = TextCursor::Position(idx + len);

                caps.render.render();
            }
            Event::Replace(from, to, text) => {
                let idx = from + text.chars().count();
                model.cursor = TextCursor::Position(idx);

                let mut change = model.note.splice_text(from, to - from, &text);

                caps.pub_sub.publish(change.bytes().to_vec());
                model.edit_timer.start(&caps.timer);

                caps.render.render();
            }
            Event::MoveCursor(idx) => {
                model.cursor = TextCursor::Position(idx);

                caps.render.render();
            }
            Event::Select(from, to) => {
                model.cursor = TextCursor::Selection(from..to);

                caps.render.render();
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
                model.edit_timer.start(&caps.timer);

                caps.render.render();
            }
            Event::ReceiveChanges(bytes) => {
                let change = Change::from_bytes(bytes).expect("a valid change");
                let mut observer = CursorObserver {
                    cursor: model.cursor.clone(),
                };

                model.note.apply_changes_with([change], &mut observer);
                model.cursor = observer.cursor;

                caps.render.render();
            }
            Event::EditTimer(TimerOutput::Created { id }) => {
                model.edit_timer.was_created(id);
            }
            Event::EditTimer(TimerOutput::Finished { id }) => {
                model.edit_timer.finished(id);

                caps.key_value
                    .write("note", model.note.save(), Event::Written);
            }
            Event::Written(_) => {
                // FIXME assuming successful write
            }
            Event::Open => caps.key_value.read("note", Event::Load),
            Event::Load(KeyValueOutput::Read(Some(data))) => {
                model.note = Note::load(&data);

                caps.pub_sub.subscribe(Event::ReceiveChanges);
                caps.render.render();
            }
            Event::Load(KeyValueOutput::Read(None)) => {
                model.note = Note::new();

                caps.key_value
                    .write("note", model.note.save(), Event::Written);
                caps.pub_sub.subscribe(Event::ReceiveChanges);

                caps.render.render();
            }
            Event::Load(KeyValueOutput::Write(_)) => unreachable!(),
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        model.into()
    }
}

struct CursorObserver {
    cursor: TextCursor,
}

impl EditObserver for CursorObserver {
    fn body_insert(&mut self, loc: usize, len: usize, _text: &str) {
        self.update_cursor(loc, len as isize);
    }

    fn body_remove(&mut self, loc: usize, len: usize) {
        self.update_cursor(loc, -(len as isize));
    }
}

impl CursorObserver {
    fn update_cursor(&mut self, loc: usize, delta: isize) {
        self.cursor = match &self.cursor {
            TextCursor::Position(position) => {
                let pos = *position as isize;

                if loc < *position {
                    TextCursor::Position((pos + delta) as usize)
                } else {
                    self.cursor.clone()
                }
            }
            TextCursor::Selection(range) => {
                let (start, end) = (range.start as isize, range.end as isize);

                match range {
                    _ if loc < range.start => {
                        let new_range = ((start + delta) as usize)..((end + delta) as usize);

                        TextCursor::Selection(new_range)
                    }
                    _ if loc >= range.start && loc < range.end => {
                        let new_range = range.start..((end + delta) as usize);

                        TextCursor::Selection(new_range)
                    }
                    _ => self.cursor.clone(),
                }
            }
        };
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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

    #[test]
    fn handles_emoji() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            // the emoji has a skintone modifier, which is a separate unicode character
            note: Note::with_text("Hello 🙌🏻 world."),
            cursor: TextCursor::Selection(3..12),
            ..Default::default()
        };

        // Replace the ' w' after the emoji
        let update = app.update(Event::Replace(8, 10, "🥳🙌🏻 w".to_string()), &mut model);
        let expected_effect = Effect::Render(RenderOperation);

        let view = app.view(&model);

        assert_eq!(view.text, "Hello 🙌🏻🥳🙌🏻 world.".to_string());
        assert_eq!(view.cursor, TextCursor::Position(13));

        assert!(
            update.effects.iter().any(|e| e == &expected_effect),
            "didn't render"
        );
    }
}

#[cfg(test)]
mod save_load_tests {
    use crux_core::testing::AppTester;
    use crux_kv::KeyValueOperation;

    use crate::capabilities::timer::{TimerOperation, TimerOutput};

    use super::*;

    #[test]
    fn opens_a_document() {
        let app = AppTester::<NoteEditor, _>::default();
        let mut note = Note::with_text("LOADED");

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Selection(2..4),
            ..Default::default()
        };

        // this will eventually take a document ID
        let update = app.update(Event::Open, &mut model);
        let key_value_effs = update
            .effects
            .iter()
            .filter(|e| matches!(e.as_ref(), Effect::KeyValue(_op)))
            .collect::<Vec<_>>();

        assert_eq!(key_value_effs.len(), 1);
        assert!(
            matches!(key_value_effs[0].as_ref(), Effect::KeyValue(KeyValueOperation::Read(key)) if key == &"note".to_string()),
            "Expected a read with key 'note', got {:?}",
            key_value_effs[0].as_ref()
        );

        // Read was successful
        let update = key_value_effs[0].resolve(&KeyValueOutput::Read(Some(note.save())));
        assert_eq!(update.events.len(), 1);

        for e in update.events {
            let update = app.update(e, &mut model);
            let renders = update
                .effects
                .iter()
                .any(|e| matches!(e.as_ref(), Effect::Render(_)));

            assert!(renders)
        }

        assert_eq!(app.view(&model).text, "LOADED");
    }

    #[test]
    fn creates_a_document_if_it_cant_open_one() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Selection(2..4),
            ..Default::default()
        };

        // this will eventually take a document ID
        let update = app.update(Event::Open, &mut model);
        let key_value_effs = update
            .effects
            .iter()
            .filter(|e| matches!(e.as_ref(), Effect::KeyValue(_op)))
            .collect::<Vec<_>>();

        assert_eq!(key_value_effs.len(), 1);
        assert!(
            matches!(key_value_effs[0].as_ref(), Effect::KeyValue(KeyValueOperation::Read(key)) if key == &"note".to_string()),
            "Expected a read with key 'note', got {:?}",
            key_value_effs[0].as_ref()
        );

        // Read was unsuccsessful
        let update = key_value_effs[0].resolve(&KeyValueOutput::Read(None));
        assert_eq!(update.events.len(), 1);

        for e in update.events {
            let update = app.update(e, &mut model);
            let saves = update
                .effects
                .iter()
                .any(|e| matches!(e.as_ref(), Effect::KeyValue(KeyValueOperation::Write(key, _)) if key == &"note".to_string()));

            assert!(saves)
        }
    }

    #[test]
    fn starts_a_timer_after_an_edit() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Selection(2..4),
            ..Default::default()
        };

        // An edit should trigger a timer
        let update = app.update(Event::Insert("something".to_string()), &mut model);
        let timer_effects: Vec<_> = update
            .effects
            .iter()
            .filter(|e| matches!(e.as_ref(), Effect::Timer(_)))
            .collect();

        assert_eq!(timer_effects.len(), 1);

        let first_id = match timer_effects[0].as_ref() {
            Effect::Timer(TimerOperation::Start { id, millis }) => {
                assert_eq!(*millis, 1000);

                id
            }
            _ => unreachable!(),
        };

        // Tells app the timer was created
        let update = timer_effects[0].resolve(&TimerOutput::Created { id: *first_id });
        for event in update.events {
            println!("Event: {event:?}");
            app.update(event, &mut model);
        }

        // Before the timer fires, insert another character, which should
        // cancel the timer and start a new one
        let update = app.update(Event::Replace(1, 2, "a".to_string()), &mut model);

        let timer_effects: Vec<_> = update
            .effects
            .iter()
            .filter(|e| matches!(e.as_ref(), Effect::Timer(_)))
            .collect();

        assert_eq!(timer_effects.len(), 2);

        let cancel = timer_effects[0];
        let start = timer_effects[1];

        let cancel_id = match cancel.as_ref() {
            Effect::Timer(TimerOperation::Cancel { id }) => id,
            _ => unreachable!(),
        };

        assert_eq!(cancel_id, first_id);

        let second_id = match start.as_ref() {
            Effect::Timer(TimerOperation::Start { id, millis }) => {
                assert_eq!(*millis, 1000);

                id
            }
            _ => unreachable!(),
        };

        assert_ne!(first_id, second_id);

        // Tell app the second timer was created
        let update = timer_effects[1].resolve(&TimerOutput::Created { id: *second_id });
        for event in update.events {
            println!("Event: {event:?}");
            app.update(event, &mut model);
        }

        // Time passes

        // Fire the timer
        let update = timer_effects[1].resolve(&TimerOutput::Finished { id: *second_id });
        for event in update.events {
            println!("Event: {event:?}");
            app.update(event, &mut model);
        }

        // One more edit. Should result in a timer, but not in cancellation
        let update = app.update(Event::Backspace, &mut model);
        let timer_effects: Vec<_> = update
            .effects
            .iter()
            .filter(|e| matches!(e.as_ref(), Effect::Timer(_)))
            .collect();

        assert_eq!(timer_effects.len(), 1);

        let third_id = match timer_effects[0].as_ref() {
            Effect::Timer(TimerOperation::Start { id, millis }) => {
                assert_eq!(*millis, 1000);

                id
            }
            _ => unreachable!(),
        };

        println!("Third id: {third_id}, second id: {second_id}");

        assert_ne!(third_id, second_id);
    }

    #[test]
    fn saves_document_when_typing_stops() {
        let app = AppTester::<NoteEditor, _>::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(5),
            edit_timer: EditTimer {
                current_id: Some(1),
                next_id: 2,
            },
        };

        // An edit should trigger a timer
        let update = app.update(
            Event::EditTimer(TimerOutput::Finished { id: 1 }),
            &mut model,
        );
        let write_effect = update
            .effects
            .iter()
            .find(|e| matches!(e.as_ref(), Effect::KeyValue(KeyValueOperation::Write(_, _))))
            .expect("a key value write");

        if let Effect::KeyValue(KeyValueOperation::Write(key, value)) = write_effect.as_ref() {
            assert_eq!(key, &"note".to_string());
            assert_eq!(value, &model.note.save());
        } else {
            unreachable!();
        }
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

        alice.update(Event::Load(KeyValueOutput::Read(Some(note.clone()))));
        bob.update(Event::Load(KeyValueOutput::Read(Some(note))));

        (alice, bob)
    }

    #[test]
    fn one_way_sync() {
        let (mut alice, mut bob) = make_alice_and_bob();

        alice.update(Event::Insert("Hello".to_string()));
        let edits = alice.edits.drain(0..).collect::<Vec<_>>();

        bob.send_edits(edits.as_ref());

        let alice_view = alice.view();
        let bob_view = bob.view();

        assert_eq!(alice_view.text, bob_view.text);
    }

    #[test]
    fn two_way_sync() {
        let (mut alice, mut bob) = make_alice_and_bob();

        alice.update(Event::Insert("world".to_string()));
        let edits = alice.edits.drain(0..).collect::<Vec<_>>();

        bob.send_edits(edits.as_ref());

        // Alice's inserts should go in front of Bob's cursor
        // so we break the ambiguity of same cursor position
        // as quickly as possible
        bob.update(Event::Insert("Hello ".to_string()));
        let edits = bob.edits.drain(0..).collect::<Vec<_>>();

        alice.send_edits(edits.as_ref());

        let alice_view = alice.view();
        let bob_view = bob.view();

        assert_eq!(alice_view.text, "Hello world".to_string());
        assert_eq!(alice_view.text, bob_view.text);
    }

    #[test]
    fn receiving_own_edits() {
        let (mut alice, mut bob) = make_alice_and_bob();

        alice.update(Event::Insert("world".to_string()));
        let edits = alice.edits.drain(0..).collect::<Vec<_>>();

        bob.send_edits(edits.as_ref());
        alice.send_edits(edits.as_ref());

        // Alice's inserts should go in front of Bob's cursor
        // so we break the ambiguity of same cursor position
        // as quickly as possible
        bob.update(Event::Insert("Hello ".to_string()));
        let edits = bob.edits.drain(0..).collect::<Vec<_>>();

        alice.send_edits(edits.as_ref());
        bob.send_edits(edits.as_ref());

        let alice_view = alice.view();
        let bob_view = bob.view();

        assert_eq!(alice_view.text, "Hello world".to_string());
        assert_eq!(alice_view.text, bob_view.text);
    }

    #[test]
    fn remote_insert_behind_cursor() {
        let (mut alice, mut bob) = make_alice_and_bob();

        alice.update(Event::Insert("world".to_string()));
        let edits = alice.edits.drain(0..).collect::<Vec<_>>();
        bob.send_edits(edits.as_ref());

        // Alice's inserts should go in front of Bob's cursor
        // so we break the ambiguity of same cursor position
        // as quickly as possible
        bob.update(Event::Insert("Hello ".to_string()));
        let edits = bob.edits.drain(0..).collect::<Vec<_>>();
        alice.send_edits(edits.as_ref());

        // Alice's cursor position should stay
        // at the end of the text where she last inserted
        alice.update(Event::Insert("!".to_string()));
        let edits = alice.edits.drain(0..).collect::<Vec<_>>();
        bob.send_edits(edits.as_ref());

        // So should bob's
        bob.update(Event::Insert("dear ".to_string()));
        let edits = bob.edits.drain(0..).collect::<Vec<_>>();
        alice.send_edits(edits.as_ref());

        let alice_view = alice.view();
        let bob_view = bob.view();

        assert_eq!(alice_view.text, "Hello dear world!".to_string());
        assert_eq!(alice_view.text, bob_view.text);
    }

    #[test]
    fn concurrent_conflicting_edits() {
        let (mut alice, mut bob) = make_alice_and_bob();

        alice.update(Event::Insert("Hel".to_string()));
        alice.update(Event::Insert("lo ".to_string()));

        bob.update(Event::Insert("world.".to_string()));
        bob.update(Event::Replace(5, 6, "!".to_string()));

        let alice_edits = alice.edits.drain(0..).collect::<Vec<_>>();
        let bob_edits = bob.edits.drain(0..).collect::<Vec<_>>();

        bob.send_edits(alice_edits.as_ref());
        alice.send_edits(bob_edits.as_ref());

        let alice_view = alice.view();
        let bob_view = bob.view();

        // Cannot assert on the result here, it's a conflicting change
        assert_eq!(alice_view.text, bob_view.text);
    }

    #[test]
    fn concurrent_clean_edits() {
        let (mut alice, mut bob) = make_alice_and_bob();

        alice.update(Event::Insert("hel".to_string()));
        alice.update(Event::Insert("lo ".to_string()));

        let alice_edits = alice.edits.drain(0..).collect::<Vec<_>>();
        bob.send_edits(alice_edits.as_ref());

        alice.update(Event::Replace(0, 1, "H".to_string()));

        bob.update(Event::MoveCursor(6));
        bob.update(Event::Insert("world.".to_string()));
        bob.update(Event::Backspace);
        bob.update(Event::Insert("!".to_string()));

        let alice_edits = alice.edits.drain(0..).collect::<Vec<_>>();
        let bob_edits = bob.edits.drain(0..).collect::<Vec<_>>();

        bob.send_edits(alice_edits.as_ref());
        alice.send_edits(bob_edits.as_ref());

        let alice_view = alice.view();
        let bob_view = bob.view();

        assert_eq!(alice_view.text, "Hello world!".to_string());
        assert_eq!(alice_view.text, bob_view.text);
    }

    #[test]
    fn remote_delete_moves_cursor() {
        let (mut alice, mut bob) = make_alice_and_bob();

        alice.update(Event::Insert("hel".to_string()));
        alice.update(Event::Insert("lo ".to_string()));

        let alice_edits = alice.edits.drain(0..).collect::<Vec<_>>();
        bob.send_edits(alice_edits.as_ref());

        bob.update(Event::Replace(6, 6, "world".to_string()));
        bob.update(Event::Replace(0, 1, "H".to_string()));
        let bob_edits = bob.edits.drain(0..).collect::<Vec<_>>();

        alice.send_edits(bob_edits.as_ref());

        // Alice's cursor should still be right after 'hello '
        alice.update(Event::Insert("dear ".to_string()));
        let alice_edits = alice.edits.drain(0..).collect::<Vec<_>>();

        bob.send_edits(alice_edits.as_ref());

        let alice_view = alice.view();
        let bob_view = bob.view();

        assert_eq!(alice_view.text, "Hello dear world".to_string());
        assert_eq!(alice_view.text, bob_view.text);
    }
}
