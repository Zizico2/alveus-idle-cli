//! Presentation-neutral state shared by menu domains and their UI adapters.

use bevy_ecs::component::Component;
use bevy_reflect::Reflect;

/// An ordered set of menu options with one authoritative cursor.
///
/// Transient menus can attach a specialization directly to their list entity;
/// long-lived gameplay menus can embed it in a reflected domain resource.
#[derive(Component, Debug, Clone, PartialEq, Eq, Reflect)]
pub struct ListMenuState<T> {
    pub options: Vec<T>,
    pub cursor: usize,
}

impl<T> Default for ListMenuState<T> {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            cursor: 0,
        }
    }
}

impl<T> ListMenuState<T> {
    pub fn new(options: impl IntoIterator<Item = T>) -> Self {
        Self {
            options: options.into_iter().collect(),
            cursor: 0,
        }
    }

    pub fn selected(&self) -> Option<&T> {
        self.options.get(self.cursor)
    }

    pub fn move_cursor(&mut self, delta: i32) -> bool {
        if self.options.is_empty() {
            return false;
        }
        let len = self.options.len() as i32;
        let next = (self.cursor as i32 + delta).rem_euclid(len) as usize;
        self.set_cursor(next)
    }

    pub fn set_cursor(&mut self, index: usize) -> bool {
        if index >= self.options.len() || self.cursor == index {
            return false;
        }
        self.cursor = index;
        true
    }

    pub fn selected_index(&self) -> Option<usize> {
        (!self.options.is_empty() && self.cursor < self.options.len()).then_some(self.cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::ListMenuState;

    #[test]
    fn cursor_wraps_and_empty_lists_are_stable() {
        let mut menu = ListMenuState::new(["one", "two", "three"]);
        assert!(menu.move_cursor(-1));
        assert_eq!(menu.cursor, 2);
        assert_eq!(menu.selected(), Some(&"three"));
        assert!(menu.move_cursor(1));
        assert_eq!(menu.cursor, 0);

        let mut empty = ListMenuState::<()>::default();
        assert!(!empty.move_cursor(1));
        assert!(!empty.set_cursor(0));
        assert_eq!(empty.selected_index(), None);
    }

    #[test]
    fn set_cursor_rejects_unchanged_and_out_of_bounds_indices() {
        let mut menu = ListMenuState::new([10, 20]);
        assert!(!menu.set_cursor(0));
        assert!(!menu.set_cursor(2));
        assert!(menu.set_cursor(1));
        assert_eq!(menu.selected(), Some(&20));
    }
}
