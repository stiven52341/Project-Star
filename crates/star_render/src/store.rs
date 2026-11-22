//! Storage of the GPU resources a renderer creates, addressed by index.

/// Owns resources of one kind; each is found by the position it was stored at.
pub(crate) struct Store<T> {
    items: Vec<T>,
}

impl<T> Store<T> {
    /// Keeps `item` and returns its position.
    pub(crate) fn insert(&mut self, item: T) -> usize {
        self.items.push(item);
        self.items.len() - 1
    }

    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    /// Puts `item` in place of the one at `index`; `None` if there is none.
    pub(crate) fn replace(&mut self, index: usize, item: T) -> Option<T> {
        self.items
            .get_mut(index)
            .map(|slot| std::mem::replace(slot, item))
    }
}

impl<T> Default for Store<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_are_found_by_their_position() {
        let mut store = Store::default();

        let first = store.insert("a");
        let second = store.insert("b");

        assert_eq!(store.get(first), Some(&"a"));
        assert_eq!(store.get(second), Some(&"b"));
        assert_eq!(store.get(2), None);
    }

    #[test]
    fn replaced_items_keep_their_position() {
        let mut store = Store::default();
        let index = store.insert("old");

        assert_eq!(store.replace(index, "new"), Some("old"));
        assert_eq!(store.get(index), Some(&"new"));
        assert_eq!(store.replace(1, "nowhere"), None);
    }
}
