use rustdoc_types::{ItemKind, ItemSummary};

pub fn is_relevant(summary: &ItemSummary) -> bool {
    matches!(summary.kind, ItemKind::Struct | ItemKind::Enum)
}
