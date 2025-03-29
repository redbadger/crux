use std::cmp::Ordering;

/// An indexed value.
/// Used for preserving member position in parent type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Indexed<T> {
    pub index: u32,
    pub value: T,
}

impl<T: Clone> Indexed<T> {
    pub(crate) fn inner(&self) -> T {
        self.value.clone()
    }
}

impl<T: Eq> Ord for Indexed<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

impl<T: Eq> PartialOrd for Indexed<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
