// The split layout of one workspace, as a tree.
//
// Plain Rust, no GTK. Each node is either a pane (one terminal) or a split
// with exactly two children. The GTK view turns this tree into real widgets.
//
//   ┌─────┬─────┐            Split (Right)
//   │     │  2  │           /             \
//   │  1  ├─────┤      Pane 1          Split (Down)
//   │     │  3  │                      /         \
//   └─────┴─────┘                 Pane 2       Pane 3

// A unique number for each pane, so we can tell panes apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneId(u32);

// Which way a split goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Right, // the new pane goes to the right (side by side)
    Down,  // the new pane goes below (top and bottom)
}

// One node of the tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Pane(PaneId),
    Split {
        direction: SplitDirection,
        first: Box<Node>,  // left or top
        second: Box<Node>, // right or bottom
    },
}

// What happened when we tried to close a pane.
#[derive(Debug, PartialEq, Eq)]
pub enum CloseResult {
    Closed,
    // It is the only pane. There is nothing to give its space to, so the
    // caller decides what to do (for example close the workspace).
    LastPane,
    NotFound,
}

// The whole layout: the tree, plus a counter for new pane IDs.
#[derive(Debug)]
pub struct Layout {
    root: Node,
    next_id: u32,
}

impl Layout {
    // A new layout with a single pane.
    pub fn new() -> Self {
        Self {
            root: Node::Pane(PaneId(0)),
            next_id: 1,
        }
    }

    pub fn root(&self) -> &Node {
        &self.root
    }

    // All pane IDs, from left/top to right/bottom.
    pub fn panes(&self) -> Vec<PaneId> {
        let mut panes = Vec::new();
        self.root.collect_panes(&mut panes);
        panes
    }

    // Split `pane` in two. The new pane goes to the right or below.
    // Returns the new pane's ID, or `None` if `pane` is not in the layout.
    pub fn split(&mut self, pane: PaneId, direction: SplitDirection) -> Option<PaneId> {
        let new_pane = PaneId(self.next_id);
        if self.root.split(pane, new_pane, direction) {
            self.next_id += 1;
            Some(new_pane)
        } else {
            None
        }
    }

    // Close `pane`. Its sibling takes over the space.
    pub fn close(&mut self, pane: PaneId) -> CloseResult {
        if self.root == Node::Pane(pane) {
            return CloseResult::LastPane;
        }
        if self.root.remove(pane) {
            CloseResult::Closed
        } else {
            CloseResult::NotFound
        }
    }
}

impl Node {
    // Find `target` in this part of the tree and replace it with a split
    // holding `target` and `new_pane`. Returns `true` if it was found.
    fn split(&mut self, target: PaneId, new_pane: PaneId, direction: SplitDirection) -> bool {
        match self {
            Node::Pane(id) if *id == target => {
                *self = Node::Split {
                    direction,
                    first: Box::new(Node::Pane(target)),
                    second: Box::new(Node::Pane(new_pane)),
                };
                true
            }
            Node::Pane(_) => false,
            Node::Split { first, second, .. } => {
                first.split(target, new_pane, direction)
                    || second.split(target, new_pane, direction)
            }
        }
    }

    // Add every pane ID below this node to `panes`, in order.
    fn collect_panes(&self, panes: &mut Vec<PaneId>) {
        match self {
            Node::Pane(id) => panes.push(*id),
            Node::Split { first, second, .. } => {
                first.collect_panes(panes);
                second.collect_panes(panes);
            }
        }
    }

    // Find `target` below this node and remove it: the split that holds it is
    // replaced by the other child (the sibling). Returns `true` if it was found.
    fn remove(&mut self, target: PaneId) -> bool {
        let Node::Split { first, second, .. } = self else {
            return false; // a pane has no children to remove
        };

        if **first == Node::Pane(target) {
            // Take the sibling out of its box. Rust does not allow leaving a
            // hole, so `replace` puts a throwaway placeholder in its place.
            let sibling = std::mem::replace(&mut **second, Node::Pane(target));
            *self = sibling;
            true
        } else if **second == Node::Pane(target) {
            let sibling = std::mem::replace(&mut **first, Node::Pane(target));
            *self = sibling;
            true
        } else {
            first.remove(target) || second.remove(target)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(id: u32) -> Box<Node> {
        Box::new(Node::Pane(PaneId(id)))
    }

    #[test]
    fn starts_with_one_pane() {
        let layout = Layout::new();
        assert_eq!(layout.root(), &Node::Pane(PaneId(0)));
    }

    #[test]
    fn split_makes_two_panes() {
        let mut layout = Layout::new();
        let new_pane = layout.split(PaneId(0), SplitDirection::Right);

        assert_eq!(new_pane, Some(PaneId(1)));
        assert_eq!(
            layout.root(),
            &Node::Split {
                direction: SplitDirection::Right,
                first: pane(0),
                second: pane(1),
            }
        );
    }

    #[test]
    fn split_inside_a_split() {
        // Build the layout from the picture at the top of this file.
        let mut layout = Layout::new();
        layout.split(PaneId(0), SplitDirection::Right); // 0 | 1
        layout.split(PaneId(1), SplitDirection::Down); //  1 becomes 1 over 2

        assert_eq!(
            layout.root(),
            &Node::Split {
                direction: SplitDirection::Right,
                first: pane(0),
                second: Box::new(Node::Split {
                    direction: SplitDirection::Down,
                    first: pane(1),
                    second: pane(2),
                }),
            }
        );
    }

    #[test]
    fn close_gives_space_to_sibling() {
        let mut layout = Layout::new();
        layout.split(PaneId(0), SplitDirection::Right); // 0 | 1

        assert_eq!(layout.close(PaneId(1)), CloseResult::Closed);
        assert_eq!(layout.root(), &Node::Pane(PaneId(0)));
    }

    #[test]
    fn close_inside_a_split() {
        // 0 | (1 over 2), then close 2: should become 0 | 1
        let mut layout = Layout::new();
        layout.split(PaneId(0), SplitDirection::Right);
        layout.split(PaneId(1), SplitDirection::Down);

        assert_eq!(layout.close(PaneId(2)), CloseResult::Closed);
        assert_eq!(
            layout.root(),
            &Node::Split {
                direction: SplitDirection::Right,
                first: pane(0),
                second: pane(1),
            }
        );
    }

    #[test]
    fn close_keeps_sibling_subtree() {
        // 0 | (1 over 2), then close 0: the whole (1 over 2) takes over
        let mut layout = Layout::new();
        layout.split(PaneId(0), SplitDirection::Right);
        layout.split(PaneId(1), SplitDirection::Down);

        assert_eq!(layout.close(PaneId(0)), CloseResult::Closed);
        assert_eq!(
            layout.root(),
            &Node::Split {
                direction: SplitDirection::Down,
                first: pane(1),
                second: pane(2),
            }
        );
    }

    #[test]
    fn closing_the_last_pane_is_reported() {
        let mut layout = Layout::new();
        assert_eq!(layout.close(PaneId(0)), CloseResult::LastPane);
        assert_eq!(layout.root(), &Node::Pane(PaneId(0)));
    }

    #[test]
    fn close_unknown_pane_is_reported() {
        let mut layout = Layout::new();
        layout.split(PaneId(0), SplitDirection::Right);
        assert_eq!(layout.close(PaneId(42)), CloseResult::NotFound);
    }

    #[test]
    fn panes_lists_all_in_order() {
        let mut layout = Layout::new();
        layout.split(PaneId(0), SplitDirection::Right);
        layout.split(PaneId(1), SplitDirection::Down);
        assert_eq!(layout.panes(), vec![PaneId(0), PaneId(1), PaneId(2)]);
    }

    #[test]
    fn split_unknown_pane_does_nothing() {
        let mut layout = Layout::new();
        assert_eq!(layout.split(PaneId(42), SplitDirection::Right), None);
        assert_eq!(layout.root(), &Node::Pane(PaneId(0)));
    }
}
