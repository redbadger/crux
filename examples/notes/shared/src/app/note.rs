use automerge::{
    transaction::Transactable, Automerge, Change, ObjId, ObjType, OpObserver, ReadDoc, ROOT,
};

pub struct Note {
    document: Automerge,
}

impl Default for Note {
    fn default() -> Self {
        Self::new()
    }
}

impl Note {
    pub fn new() -> Self {
        let mut document = Automerge::new();

        document
            .transact(|tx| tx.put_object(automerge::ROOT, "body", ObjType::Text))
            .expect("to create a document");

        Self {
            document: document.fork(),
        }
    }

    pub fn with_text(text: &str) -> Self {
        let mut note = Self::new();
        let body = note.body();

        note.document
            .transact(|tx| tx.splice_text(&body, 0, 0, text))
            .expect("to update body of a new note");

        assert_eq!(note.text(), text.to_string());

        note
    }

    pub fn save(&mut self) -> Vec<u8> {
        self.document.save()
    }

    pub fn load(bytes: &[u8]) -> Self {
        let document = Automerge::load(bytes).expect("to load document");

        Self { document }
    }

    pub fn text(&self) -> String {
        self.document
            .text(self.body())
            .expect("document to have body")
    }

    pub fn length(&self) -> usize {
        self.document.length(self.body())
    }

    pub fn splice_text(&mut self, pos: usize, del: usize, text: &str) -> Change {
        let body = self.body();

        println!("Splice {pos} {del} '{text}'");

        self.document
            .transact(|tx| tx.splice_text(body, pos, del, text))
            .expect("to splice the body text");

        self.document
            .get_last_local_change()
            .expect("to find a change")
            .clone()
    }

    pub fn apply_changes(&mut self, changes: impl IntoIterator<Item = Change>) {
        self.document
            .apply_changes(changes)
            .expect("to apply changes")
    }

    pub fn apply_changes_with(
        &mut self,
        changes: impl IntoIterator<Item = Change>,
        edit_observer: &mut impl EditObserver,
    ) {
        let mut observer = Observer { edit_observer };

        self.document
            .apply_changes_with(changes, Some(&mut observer))
            .expect("to apply changes")
    }

    fn body(&self) -> ObjId {
        self.document
            .get(ROOT, "body")
            .expect("to get")
            .expect("to find body")
            .1
    }
}

pub trait EditObserver {
    fn body_insert(&mut self, loc: usize, len: usize, text: &str);
    fn body_remove(&mut self, loc: usize, len: usize);
}

struct Observer<'a> {
    edit_observer: &'a mut dyn EditObserver,
}

impl OpObserver for Observer<'_> {
    fn insert<R: automerge::ReadDoc>(
        &mut self,
        _doc: &R,
        _objid: automerge::ObjId,
        _index: usize,
        _tagged_value: (automerge::Value<'_>, automerge::ObjId),
        _conflict: bool,
    ) {
        // not interested
    }

    fn splice_text<R: automerge::ReadDoc>(
        &mut self,
        _doc: &R,
        _objid: automerge::ObjId,
        index: usize,
        value: &str,
    ) {
        self.edit_observer.body_insert(index, value.len(), value);
    }

    fn put<R: automerge::ReadDoc>(
        &mut self,
        _doc: &R,
        _objid: automerge::ObjId,
        _prop: automerge::Prop,
        _tagged_value: (automerge::Value<'_>, automerge::ObjId),
        _conflict: bool,
    ) {
        // not interested
    }

    fn expose<R: automerge::ReadDoc>(
        &mut self,
        _doc: &R,
        _objid: automerge::ObjId,
        _prop: automerge::Prop,
        _tagged_value: (automerge::Value<'_>, automerge::ObjId),
        _conflict: bool,
    ) {
        // not interested
    }

    fn increment<R: automerge::ReadDoc>(
        &mut self,
        _doc: &R,
        _objid: automerge::ObjId,
        _prop: automerge::Prop,
        _tagged_value: (i64, automerge::ObjId),
    ) {
        // not interested
    }

    fn delete_map<R: automerge::ReadDoc>(
        &mut self,
        _doc: &R,
        _objid: automerge::ObjId,
        _key: &str,
    ) {
        // not interested
    }

    fn delete_seq<R: automerge::ReadDoc>(
        &mut self,
        _doc: &R,
        _objid: automerge::ObjId,
        index: usize,
        num: usize,
    ) {
        self.edit_observer.body_remove(index, num);
    }

    fn mark<'b, R: ReadDoc, M: Iterator<Item = automerge::marks::Mark<'b>>>(
        &mut self,
        _doc: &'b R,
        _objid: ObjId,
        _mark: M,
    ) {
        // not interested
    }

    fn unmark<R: ReadDoc>(
        &mut self,
        _doc: &R,
        _objid: ObjId,
        _name: &str,
        _start: usize,
        _end: usize,
    ) {
        // not interested
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn inserts_text() {
        let mut note = Note::new();

        note.splice_text(0, 0, "hello");

        assert_eq!(note.text(), "hello");
    }

    #[test]
    fn splices_text() {
        let mut note = Note::with_text("hello");

        note.splice_text(2, 1, "L");

        assert_eq!(note.text(), "heLlo");
    }
}
