mod note;

use std::{ops::Range, time::Duration};

use automerge::Change;
use crux_core::{
    macros::effect,
    render::{self, RenderOperation},
    App, Command,
};
use crux_kv::{command::KeyValue, error::KeyValueError, KeyValueOperation};
use crux_time::{
    command::{Time, TimerHandle, TimerOutcome},
    TimeRequest,
};
use serde::{Deserialize, Serialize};

use crate::capabilities::pub_sub::{PubSub, PubSubOperation};

pub use note::Note;

use self::note::EditObserver;

#[derive(Default)]
pub struct NoteEditor;

#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    // events from the shell
    Open,
    Insert(String),
    Replace(usize, usize, String),
    MoveCursor(usize),
    Select(usize, usize),
    Backspace,
    Delete,
    ReceiveChanges(Vec<u8>),

    // events local to the core
    #[serde(skip)]
    EditTimerElapsed(TimerOutcome),
    #[serde(skip)]
    Written(Result<Option<Vec<u8>>, KeyValueError>),
    #[serde(skip)]
    Load(Result<Option<Vec<u8>>, KeyValueError>),
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
    timer: Option<TimerHandle>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ViewModel {
    pub text: String,
    pub cursor: TextCursor,
}

impl From<&Model> for ViewModel {
    fn from(model: &Model) -> Self {
        ViewModel {
            text: model.note.text(),
            cursor: model.cursor.clone(),
        }
    }
}

#[effect(typegen)]
pub enum Effect {
    Time(TimeRequest),
    Render(RenderOperation),
    PubSub(PubSubOperation),
    KeyValue(KeyValueOperation),
}

const EDIT_TIMER: u64 = 1000;

impl App for NoteEditor {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    type Capabilities = ();

    // ANCHOR: update
    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        _caps: &Self::Capabilities,
    ) -> Command<Effect, Event> {
        // delegate to our own update method for testing. This will not be necessary
        // once the `App` trait has been modified to remove the `caps` parameter.
        self.update(event, model)
    }
    // ANCHOR_END: update

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        model.into()
    }
}

impl NoteEditor {
    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
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

                let publish = PubSub::publish(change.bytes().to_vec()).into();

                let len = text.chars().count();
                let idx = match &model.cursor {
                    TextCursor::Position(idx) => *idx,
                    TextCursor::Selection(range) => range.start,
                };
                model.cursor = TextCursor::Position(idx + len);

                Command::all(vec![
                    publish,
                    restart_timer(&mut model.timer),
                    render::render(),
                ])
            }
            Event::Replace(from, to, text) => {
                let idx = from + text.chars().count();
                model.cursor = TextCursor::Position(idx);

                let mut change = model.note.splice_text(from, to - from, &text);
                let publish = PubSub::publish(change.bytes().to_vec()).into();

                Command::all(vec![
                    publish,
                    restart_timer(&mut model.timer),
                    render::render(),
                ])
            }
            Event::MoveCursor(idx) => {
                model.cursor = TextCursor::Position(idx);

                render::render()
            }
            Event::Select(from, to) => {
                model.cursor = TextCursor::Selection(from..to);

                render::render()
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
                let publish = PubSub::publish(change.bytes().to_vec()).into();

                Command::all(vec![
                    publish,
                    restart_timer(&mut model.timer),
                    render::render(),
                ])
            }
            Event::ReceiveChanges(bytes) => {
                let change = Change::from_bytes(bytes).expect("a valid change");
                let mut observer = CursorObserver {
                    cursor: model.cursor.clone(),
                };

                model.note.apply_changes_with([change], &mut observer);
                model.cursor = observer.cursor;

                render::render()
            }
            Event::EditTimerElapsed(TimerOutcome::Completed(_)) => {
                KeyValue::set("note".to_string(), model.note.save()).then_send(Event::Written)
            }
            Event::EditTimerElapsed(TimerOutcome::Cleared) => Command::done(),
            Event::Written(_) => {
                // FIXME assuming successful write
                Command::done()
            }
            Event::Open => KeyValue::get("note".to_string()).then_send(Event::Load),
            Event::Load(Ok(value)) => {
                let mut commands = Vec::new();
                if value.is_none() {
                    model.note = Note::new();

                    commands.push(
                        KeyValue::set("note".to_string(), model.note.save())
                            .then_send(Event::Written),
                    );
                } else {
                    model.note = Note::load(&value.unwrap_or_default());
                }
                commands.push(PubSub::subscribe().then_send(Event::ReceiveChanges));

                commands.push(render::render());
                Command::all(commands)
            }
            Event::Load(Err(_)) => {
                // FIXME handle error
                Command::done()
            }
        }
    }
}

fn restart_timer(current_handle: &mut Option<TimerHandle>) -> Command<Effect, Event> {
    if let Some(handle) = current_handle.take() {
        handle.clear()
    }

    let duration = Duration::from_millis(EDIT_TIMER);
    let (notify_after, handle) = Time::notify_after(duration);
    current_handle.replace(handle);
    notify_after.then_send(Event::EditTimerElapsed)
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
    use crux_core::assert_effect;

    use super::*;

    #[test]
    fn renders_text_and_cursor() {
        let app = NoteEditor::default();

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
        let app = NoteEditor::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(3),
            ..Default::default()
        };

        let mut cmd = app.update(Event::MoveCursor(5), &mut model);
        cmd.expect_one_effect().expect_render();

        let view = app.view(&model);

        assert_eq!(view.text, "hello");
        assert_eq!(view.cursor, TextCursor::Position(5));
    }

    #[test]
    fn changes_selection() {
        let app = NoteEditor::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(3),
            ..Default::default()
        };

        let mut cmd = app.update(Event::Select(2, 5), &mut model);
        cmd.expect_one_effect().expect_render();

        let view = app.view(&model);

        assert_eq!(view.text, "hello");
        assert_eq!(view.cursor, TextCursor::Selection(2..5));
    }

    #[test]
    fn inserts_text_at_cursor_and_renders() {
        let app = NoteEditor::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(3),
            ..Default::default()
        };

        let mut cmd = app.update(Event::Insert("l to the ".to_string()), &mut model);
        assert_effect!(cmd, Effect::Render(_));

        let view = app.view(&model);

        assert_eq!(view.text, "hell to the lo");
        assert_eq!(view.cursor, TextCursor::Position(12));
    }

    // ANCHOR: replaces_selection_and_renders
    #[test]
    fn replaces_selection_and_renders() {
        let app = NoteEditor::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Selection(3..5),
            ..Default::default()
        };

        let event = Event::Insert("ter skelter".to_string());
        let mut cmd = app.update(event, &mut model);
        assert_effect!(cmd, Effect::Render(_));

        let view = app.view(&model);

        assert_eq!(view.text, "helter skelter");
        assert_eq!(view.cursor, TextCursor::Position(14));
    }
    // ANCHOR_END: replaces_selection_and_renders

    #[test]
    fn replaces_range_and_renders() {
        let app = NoteEditor::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(3),
            ..Default::default()
        };

        let mut cmd = app.update(Event::Replace(1, 4, "i, y".to_string()), &mut model);
        assert_effect!(cmd, Effect::Render(_));

        let view = app.view(&model);

        assert_eq!(view.text, "hi, yo");
        assert_eq!(view.cursor, TextCursor::Position(5));
    }

    #[test]
    fn replaces_empty_range_and_renders() {
        let app = NoteEditor::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(3),
            ..Default::default()
        };

        let mut cmd = app.update(
            Event::Replace(1, 1, "ey, just saying h".to_string()),
            &mut model,
        );
        assert_effect!(cmd, Effect::Render(_));

        let view = app.view(&model);

        assert_eq!(view.text, "hey, just saying hello");
        assert_eq!(view.cursor, TextCursor::Position(18));
    }

    #[test]
    fn removes_character_before_cursor() {
        let app = NoteEditor::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(2),
            ..Default::default()
        };

        let mut cmd = app.update(Event::Backspace, &mut model);
        assert_effect!(cmd, Effect::Render(_));

        let view = app.view(&model);

        assert_eq!(view.text, "hllo");
        assert_eq!(view.cursor, TextCursor::Position(1));
    }

    #[test]
    fn removes_character_after_cursor() {
        let app = NoteEditor::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Position(2),
            ..Default::default()
        };

        let mut cmd = app.update(Event::Delete, &mut model);
        assert_effect!(cmd, Effect::Render(_));

        let view = app.view(&model);

        assert_eq!(view.text, "helo");
        assert_eq!(view.cursor, TextCursor::Position(2));
    }

    #[test]
    fn removes_selection_on_delete() {
        let app = NoteEditor::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Selection(2..4),
            ..Default::default()
        };

        let mut cmd = app.update(Event::Delete, &mut model);
        assert_effect!(cmd, Effect::Render(_));

        let view = app.view(&model);

        assert_eq!(view.text, "heo");
        assert_eq!(view.cursor, TextCursor::Position(2));
    }

    #[test]
    fn removes_selection_on_backspace() {
        let app = NoteEditor::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Selection(2..4),
            ..Default::default()
        };

        let mut cmd = app.update(Event::Backspace, &mut model);
        assert_effect!(cmd, Effect::Render(_));

        let view = app.view(&model);

        assert_eq!(view.text, "heo");
        assert_eq!(view.cursor, TextCursor::Position(2));
    }

    #[test]
    fn handles_emoji() {
        let app = NoteEditor::default();

        let mut model = Model {
            // the emoji has a skintone modifier, which is a separate unicode character
            note: Note::with_text("Hello 🙌🏻 world."),
            cursor: TextCursor::Selection(3..12),
            ..Default::default()
        };

        // Replace the ' w' after the emoji
        let mut cmd = app.update(Event::Replace(8, 10, "🥳🙌🏻 w".to_string()), &mut model);
        assert_effect!(cmd, Effect::Render(_));

        let view = app.view(&model);

        assert_eq!(view.text, "Hello 🙌🏻🥳🙌🏻 world.");
        assert_eq!(view.cursor, TextCursor::Position(13));
    }
}

#[cfg(test)]
mod save_load_tests {
    use crux_core::assert_effect;
    use crux_kv::{value::Value, KeyValueOperation, KeyValueResponse, KeyValueResult};
    use crux_time::{TimeRequest, TimerId};

    use super::*;

    #[test]
    fn opens_a_document() {
        let app = NoteEditor::default();
        let mut note = Note::with_text("LOADED");

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Selection(2..4),
            ..Default::default()
        };

        // this will eventually take a document ID
        let mut cmd = app.update(Event::Open, &mut model);
        let mut effects = cmd.effects();

        let request = &mut effects.next().unwrap().expect_key_value();
        assert_eq!(
            request.operation,
            KeyValueOperation::Get {
                key: "note".to_string()
            }
        );

        assert!(effects.next().is_none());

        // Read was successful
        request
            .resolve(KeyValueResult::Ok {
                response: KeyValueResponse::Get {
                    value: note.save().into(),
                },
            })
            .unwrap();
        drop(effects);

        let load_event = cmd.events().next().unwrap();
        assert!(matches!(load_event, Event::Load(Ok(_))));

        let mut cmd = app.update(load_event, &mut model);
        assert_effect!(cmd, Effect::Render(_));

        assert_eq!(app.view(&model).text, "LOADED");
    }

    #[test]
    fn creates_a_document_if_it_cant_open_one() {
        let app = NoteEditor::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Selection(2..4),
            ..Default::default()
        };

        // this will eventually take a document ID
        let mut cmd = app.update(Event::Open, &mut model);
        let mut effects = cmd.effects().filter(Effect::is_key_value);

        let request = &mut effects.next().unwrap().expect_key_value();
        assert_eq!(
            request.operation,
            KeyValueOperation::Get {
                key: "note".to_string()
            }
        );

        assert!(effects.next().is_none());

        request
            .resolve(KeyValueResult::Ok {
                response: KeyValueResponse::Get { value: Value::None },
            })
            .unwrap();
        drop(effects);

        let load_event = cmd.events().next().unwrap();
        assert!(matches!(load_event, Event::Load(Ok(None))));

        let mut cmd = app.update(load_event, &mut model);
        let request = cmd.effects().find_map(Effect::into_key_value).unwrap();

        assert_eq!(
            request.operation,
            KeyValueOperation::Set {
                key: "note".to_string(),
                value: model.note.save().into(),
            }
        );
    }

    // ANCHOR: starts_a_timer_after_an_edit
    #[test]
    fn starts_a_timer_after_an_edit() {
        let app = NoteEditor::default();

        let mut model = Model {
            note: Note::with_text("hello"),
            cursor: TextCursor::Selection(2..4),
            ..Default::default()
        };

        // An edit should trigger a timer
        let event = Event::Insert("something".to_string());
        let mut cmd1 = app.update(event, &mut model);
        let mut requests = cmd1.effects().filter_map(Effect::into_time);

        let request = requests.next().unwrap();
        let (first_id, duration) = match &request.operation {
            TimeRequest::NotifyAfter { id, duration } => (id.clone(), duration),
            _ => panic!("expected a NotifyAfter"),
        };
        assert_eq!(duration, &Duration::from_secs(1).into());

        assert!(requests.next().is_none());
        drop(requests); // so we can use cmd1 later

        // Before the timer fires, insert another character, which should
        // cancel the timer and start a new one
        let mut cmd2 = app.update(Event::Replace(1, 2, "a".to_string()), &mut model);
        let mut requests = cmd2.effects().filter_map(Effect::into_time);

        // but first, the original request (cmd1) should resolve with a clear
        let cancel_request = cmd1.effects().filter_map(Effect::into_time).next().unwrap();
        let cancel_id = match &cancel_request.operation {
            TimeRequest::Clear { id } => id.clone(),
            _ => panic!("expected a Clear"),
        };
        assert_eq!(cancel_id, first_id);

        // request to start the second timer
        let mut start_request = requests.next().unwrap();
        let second_id = match &start_request.operation {
            TimeRequest::NotifyAfter { id, duration: _ } => id.clone(),
            _ => panic!("expected a NotifyAfter"),
        };

        assert_ne!(first_id, second_id);

        assert!(requests.next().is_none());
        drop(requests); // so we can use cmd2 later

        // Time passes
        start_request
            .resolve(crux_time::TimeResponse::DurationElapsed { id: second_id })
            .unwrap();

        // send the elapsed event back into the app
        let event = cmd2.events().next().unwrap();
        let mut cmd3 = app.update(event, &mut model);

        // we should see an effect to save the note
        let mut effects = cmd3.effects();
        let save = effects.next().unwrap().expect_key_value();
        assert_eq!(
            save.operation,
            KeyValueOperation::Set {
                key: "note".to_string(),
                value: model.note.save().into(),
            }
        );

        // One more edit. Should result in a new timer
        let mut cmd4 = app.update(Event::Backspace, &mut model);
        let mut effects = cmd4.effects();

        let _publish = effects.next().unwrap().expect_pub_sub();
        let timer = effects.next().unwrap().expect_time();
        assert_eq!(
            timer.operation,
            TimeRequest::NotifyAfter {
                id: TimerId(3),
                duration: crux_time::Duration::from_millis(1000)
            }
        );
    }
    // ANCHOR_END: starts_a_timer_after_an_edit
}

#[cfg(test)]
mod sync_tests {
    use std::collections::VecDeque;

    use crux_core::Request;

    use crate::capabilities::pub_sub::{Message, PubSubOperation};

    use super::*;

    struct Peer {
        app: NoteEditor,
        model: Model,
        subscription: Option<Request<PubSubOperation>>,
        command: Option<Command<Effect, Event>>,
        edits: VecDeque<Vec<u8>>,
    }

    // A jig to make testing sync a bit easier
    impl Peer {
        fn new() -> Self {
            let app = NoteEditor::default();
            let model = Default::default();

            Self {
                app,
                model,
                subscription: None,
                command: None,
                edits: VecDeque::new(),
            }
        }

        // Update, picking out and keeping PubSub effects
        fn update(&mut self, event: Event) {
            let mut cmd = self.app.update(event, &mut self.model);

            let mut subscribe = false;
            for effect in cmd.effects() {
                match effect {
                    Effect::PubSub(request) => match request.operation {
                        PubSubOperation::Subscribe => {
                            self.subscription = Some(request);
                            subscribe = true;
                        }
                        PubSubOperation::Publish(bytes) => {
                            self.edits.push_back(bytes.clone());
                        }
                    },
                    _ => (),
                }
            }
            if subscribe {
                self.command = Some(cmd);
            }
        }

        fn view(&self) -> ViewModel {
            self.app.view(&self.model)
        }

        fn send_edits(&mut self, edits: &[Vec<u8>]) {
            for edit in edits {
                print!("Sending edit: {:?}", edit);
                self.subscription
                    .as_mut()
                    .expect("to have a subscription")
                    .resolve(Message(edit.clone()))
                    .expect("should resolve");

                if let Some(cmd) = self.command.as_mut() {
                    for event in cmd.events().collect::<Vec<_>>() {
                        self.update(event);
                    }
                }
            }
        }
    }

    fn make_alice_and_bob() -> (Peer, Peer) {
        let note = Note::new().save();

        let mut alice = Peer::new();
        let mut bob = Peer::new();

        alice.update(Event::Load(Ok(Some(note.clone()))));
        bob.update(Event::Load(Ok(Some(note))));

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

        assert_eq!(alice_view.text, "Hello world");
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

        assert_eq!(alice_view.text, "Hello world");
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

        assert_eq!(alice_view.text, "Hello dear world!");
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

        assert_eq!(alice_view.text, "Hello world!");
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

        assert_eq!(alice_view.text, "Hello dear world");
        assert_eq!(alice_view.text, bob_view.text);
    }
}
