use automerge::{
    Automerge, Change, ObjId, ObjType, PatchAction, PatchLog, ROOT, ReadDoc,
    transaction::Transactable,
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
    #[must_use]
    pub fn new() -> Self {
        let mut document = Automerge::new();

        document
            .transact(|tx| tx.put_object(automerge::ROOT, "body", ObjType::Text))
            .expect("to create a document");

        Self {
            document: document.fork(),
        }
    }

    #[must_use]
    #[cfg(test)]
    pub fn with_text(text: &str) -> Self {
        let mut note = Self::new();
        let body = note.body();

        note.document
            .transact(|tx| tx.splice_text(&body, 0, 0, text))
            .expect("to update body of a new note");

        assert_eq!(note.text(), text.to_string());

        note
    }

    #[must_use]
    pub fn save(&self) -> Vec<u8> {
        self.document.save()
    }

    #[must_use]
    pub fn load(bytes: &[u8]) -> Self {
        let document = Automerge::load(bytes).expect("to load document");

        Self { document }
    }

    #[must_use]
    pub fn text(&self) -> String {
        self.document
            .text(self.body())
            .expect("document to have body")
    }

    pub fn splice_text(&mut self, pos: usize, del: usize, text: &str) -> Change {
        let body = self.body();

        println!("Splice {pos} {del} '{text}'");

        #[allow(clippy::cast_possible_wrap)]
        let del = del as isize;

        self.document
            .transact(|tx| tx.splice_text(body, pos, del, text))
            .expect("to splice the body text");

        self.document
            .get_last_local_change()
            .expect("to find a change")
    }

    pub fn apply_changes_with(
        &mut self,
        changes: impl IntoIterator<Item = Change> + Clone,
        edit_observer: &mut impl EditObserver,
    ) {
        let mut patch_log = PatchLog::active();

        self.document
            .apply_changes_log_patches(changes, &mut patch_log)
            .expect("to apply changes");

        for patch in self.document.make_patches(&mut patch_log) {
            match patch.action {
                PatchAction::SpliceText { index, value, .. } => {
                    let text = value.make_string();
                    edit_observer.body_insert(index, text.chars().count(), &text);
                }
                PatchAction::DeleteSeq { index, length } => {
                    edit_observer.body_remove(index, length);
                }
                _ => {
                    // not interested
                }
            }
        }
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
