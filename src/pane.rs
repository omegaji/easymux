// One pane's tabs: an ordered list, and which tab is shown (the active one).
//
// Plain Rust, no GTK. Each tab is one terminal; the workspace keeps the
// terminals and uses this to know which tabs a pane has, in which order.

// A unique number for each tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(pub u32);

// What happened when we tried to close a tab.
#[derive(Debug, PartialEq, Eq)]
pub enum TabCloseResult {
    Closed,
    // It is the pane's only tab. The caller decides (close the pane).
    LastTab,
    NotFound,
}

#[derive(Debug)]
pub struct Pane {
    tabs: Vec<TabId>, // in the order they appear in the tab bar
    active: TabId,    // the tab currently shown
}

impl Pane {
    // A pane with one tab.
    pub fn new(first: TabId) -> Self {
        Self {
            tabs: vec![first],
            active: first,
        }
    }

    pub fn tabs(&self) -> &[TabId] {
        &self.tabs
    }

    pub fn active(&self) -> TabId {
        self.active
    }

    // Add `tab` right after the active tab, and show it.
    pub fn add(&mut self, tab: TabId) {
        let index = self
            .index_of(self.active)
            .map_or(self.tabs.len(), |i| i + 1);
        self.tabs.insert(index, tab);
        self.active = tab;
    }

    // Show `tab`. Returns `false` if this pane has no such tab.
    pub fn activate(&mut self, tab: TabId) -> bool {
        if self.tabs.contains(&tab) {
            self.active = tab;
            true
        } else {
            false
        }
    }

    // Close `tab`. If it was shown, the tab to its right is shown instead
    // (or to its left, if it was the last one).
    pub fn close(&mut self, tab: TabId) -> TabCloseResult {
        let Some(index) = self.index_of(tab) else {
            return TabCloseResult::NotFound;
        };
        if self.tabs.len() == 1 {
            return TabCloseResult::LastTab;
        }

        self.tabs.remove(index);
        if self.active == tab {
            // `index` now points at the tab that was to the right, if any.
            let new_index = index.min(self.tabs.len() - 1);
            self.active = self.tabs[new_index];
        }
        TabCloseResult::Closed
    }

    // The position of `tab` in the list, if it is there.
    fn index_of(&self, tab: TabId) -> Option<usize> {
        self.tabs.iter().position(|t| *t == tab)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A pane with tabs 0, 1, 2 (in that order), with tab 2 active.
    fn pane_with_three_tabs() -> Pane {
        let mut pane = Pane::new(TabId(0));
        pane.add(TabId(1));
        pane.add(TabId(2));
        pane
    }

    #[test]
    fn starts_with_one_active_tab() {
        let pane = Pane::new(TabId(0));
        assert_eq!(pane.tabs(), &[TabId(0)]);
        assert_eq!(pane.active(), TabId(0));
    }

    #[test]
    fn add_goes_after_active_and_becomes_active() {
        let mut pane = pane_with_three_tabs();
        pane.activate(TabId(0));
        pane.add(TabId(3));
        assert_eq!(pane.tabs(), &[TabId(0), TabId(3), TabId(1), TabId(2)]);
        assert_eq!(pane.active(), TabId(3));
    }

    #[test]
    fn activate_unknown_tab_does_nothing() {
        let mut pane = pane_with_three_tabs();
        assert!(!pane.activate(TabId(42)));
        assert_eq!(pane.active(), TabId(2));
    }

    #[test]
    fn closing_active_tab_shows_the_one_to_its_right() {
        let mut pane = pane_with_three_tabs();
        pane.activate(TabId(1));
        assert_eq!(pane.close(TabId(1)), TabCloseResult::Closed);
        assert_eq!(pane.tabs(), &[TabId(0), TabId(2)]);
        assert_eq!(pane.active(), TabId(2));
    }

    #[test]
    fn closing_last_active_tab_shows_the_one_to_its_left() {
        let mut pane = pane_with_three_tabs(); // tab 2 (the last) is active
        assert_eq!(pane.close(TabId(2)), TabCloseResult::Closed);
        assert_eq!(pane.active(), TabId(1));
    }

    #[test]
    fn closing_a_background_tab_keeps_the_active_one() {
        let mut pane = pane_with_three_tabs(); // tab 2 is active
        assert_eq!(pane.close(TabId(0)), TabCloseResult::Closed);
        assert_eq!(pane.active(), TabId(2));
    }

    #[test]
    fn closing_the_only_tab_is_reported() {
        let mut pane = Pane::new(TabId(0));
        assert_eq!(pane.close(TabId(0)), TabCloseResult::LastTab);
        assert_eq!(pane.tabs(), &[TabId(0)]);
    }

    #[test]
    fn closing_unknown_tab_is_reported() {
        let mut pane = pane_with_three_tabs();
        assert_eq!(pane.close(TabId(42)), TabCloseResult::NotFound);
    }
}
