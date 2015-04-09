use ::Treeish;
use std::borrow::Borrow;
use std::num::{Int, SignedInt};

enum SiblingOffset {
    Root,
    Underflow,
    Overflow,
    OutOfRange(usize, usize),
    Valid(usize),
}

struct NavigatorFrame<'a, T: Treeish + 'a> {
    tree: &'a T,
    index: usize,
}

pub struct Navigator<'a, T: Treeish + 'a> {
    here: &'a T,
    path: Vec<NavigatorFrame<'a, T>>,
}

impl<'a, T: Treeish + 'a> Navigator<'a, T> {
    pub fn is_root(&self) -> bool {
        self.path.is_empty()
    }

    pub fn is_leaf(&self) -> bool {
        self.here.child_count() > 0
    }

    pub fn to_parent(&mut self) {
        match self.path.pop() {
            None => panic!["already at root"],
            Some(frame) => self.here = &frame.tree,
        }
    }

    pub fn tree<'s>(&'s self) -> &'s T {
        &self.here
    }


    pub fn seek_sibling(&mut self, offset: isize) {
        if offset == 0 {
            return;
        }
        match self.validate_sibling_offset(offset) {
            SiblingOffset::Root => panic!["tree root has no siblings"],
            SiblingOffset::Underflow => panic!["underflow computing sibling index"],
            SiblingOffset::Overflow => panic!["overflow computing sibling index"],
            SiblingOffset::OutOfRange(new_index, siblings) =>
                panic!["cannot address sibling {} (only {} siblings)",
                       new_index, siblings],
            SiblingOffset::Valid(new_index) => {
                let parent = &mut self.path[self.path.len() - 1];
                parent.index = new_index;
                self.here = &parent.tree.child(new_index);
            },
        }
    }

    pub fn seek_child(&mut self, child_index: usize) {
        // TODO
    }

    pub fn has_left(&self) -> bool {
        if self.is_root() {
            return false;
        }
        self.path[self.path.len() - 1].index > 0
    }

    pub fn has_right(&self) -> bool {
        if self.is_root() {
            return false;
        }
        let parent = &self.path[self.path.len() - 1];
        parent.index < parent.tree.child_count()
    }

    pub fn to_left(&mut self) {
        match self.path.pop() {
            None => panic!["root node has no siblings"],
            Some(mut parent) => {
                if parent.index == 0 {
                    panic!["already at leftmost sibling"];
                }
                parent.index -= 1;
                // TODO update self.here
                self.path.push(parent);
            },
        }
    }

    pub fn to_right(&mut self) {
        match self.path.pop() {
            None => panic!["root node has no siblings"],
            Some(mut parent) => {
                parent.index += 1;
                // TODO update self.here
                self.path.push(parent);
            },
        }
    }

    fn validate_sibling_offset(&self, offset: isize) -> SiblingOffset {
        if self.is_root() {
            return SiblingOffset::Root;
        }
        let offset_abs = offset.abs();
        let parent = &self.path[self.path.len() - 1];
        let new_index =
            if offset_abs < 0 {
                // offset is Int::min_value().
                match parent.index.checked_sub(1) {
                    None => return SiblingOffset::Underflow,
                    Some(x) => match x.checked_sub((offset_abs + 1isize).abs() as usize) {
                        None => return SiblingOffset::Underflow,
                        Some(x) => x,
                    },
                }
            } else {
                if offset < 0 {
                    match parent.index.checked_sub(offset_abs as usize) {
                        None => return SiblingOffset::Underflow,
                        Some(x) => x,
                    }
                } else {
                    match parent.index.checked_add(offset_abs as usize) {
                        None => return SiblingOffset::Overflow,
                        Some(x) => x,
                    }
                }
            };
        let child_count = parent.tree.child_count();
        if new_index >= child_count {
            SiblingOffset::OutOfRange(new_index, child_count)
        } else {
            SiblingOffset::Valid(new_index)
        }
    }
}

impl<'a, T: Treeish> Borrow<T> for Navigator<'a, T> {
    fn borrow(&self) -> &T {
        self.here
    }
}
