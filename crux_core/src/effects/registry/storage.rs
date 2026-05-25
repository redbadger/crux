use slab::Slab;

use super::effect_id;
use crate::RequestHandle;

pub(crate) struct Storage<Out> {
    pub(crate) handles: Slab<Option<RequestHandle<Out>>>,
    pub(crate) generations: Vec<u32>,
}

impl<Out> Default for Storage<Out> {
    fn default() -> Self {
        Self {
            handles: Slab::with_capacity(256),
            generations: Vec::with_capacity(256),
        }
    }
}

impl<Out> Storage<Out> {
    pub(crate) fn insert(&mut self, handle: RequestHandle<Out>) -> effect_id::EffectId<Out> {
        let entry = self.handles.vacant_entry();
        let index = entry.key();

        if self.generations.len() == index {
            self.generations.push(0);
        }
        debug_assert!(self.generations.len() > index);

        let generation = self.generations[index];
        entry.insert(Some(handle));

        effect_id::EffectId::new(index, generation)
    }

    pub(crate) fn take(&mut self, id: effect_id::EffectId<Out>) -> Option<RequestHandle<Out>> {
        if self.generations.get(id.index()).copied()? != id.generation() {
            return None;
        }

        self.handles.get_mut(id.index())?.take()
    }

    pub(crate) fn reinsert(&mut self, id: effect_id::EffectId<Out>, handle: RequestHandle<Out>) {
        let Some(entry) = self.handles.get_mut(id.index()) else {
            return;
        };

        debug_assert_eq!(self.generations[id.index()], id.generation());
        debug_assert!(entry.is_none());

        *entry = Some(handle);
    }

    pub(crate) fn remove(&mut self, id: effect_id::EffectId<Out>) {
        if self.generations.get(id.index()).copied() != Some(id.generation()) {
            return;
        }

        if self.handles.contains(id.index()) {
            self.handles.remove(id.index());
            self.generations[id.index()] = self.generations[id.index()].wrapping_add(1);
        }
    }
}
