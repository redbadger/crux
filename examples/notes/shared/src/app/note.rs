use automerge::{transaction::Transactable, Automerge, Change, ObjId, ObjType, ReadDoc, ROOT};

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

    pub fn splice_text(&mut self, pos: usize, del: usize, text: &str) -> Change {
        let body = self.body();

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

    fn body(&self) -> ObjId {
        self.document
            .get(ROOT, "body")
            .expect("to get")
            .expect("to find body")
            .1
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
