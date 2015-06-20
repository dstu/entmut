use ::Nav;
use ::traversal::Queue;
use ::util::{ChildIndex, SiblingIndex};

use std::borrow::{Borrow, BorrowMut};
use std::clone::Clone;
use std::iter::Iterator;

/// Fixed-layout tree with good memory locality guarantees.
///
/// This tree structure does not provide methods for arbitrarily modifying its
/// topology, but it does provide good memory locality guarantees. Internally,
/// tree data is kept in a single heap-allocated region. Records of tree edges
/// are also kept in contiguous regions of memory, so tree navigation should be
/// fast.
///
/// If the tree is extended with additional children, it may reallocate its
/// entire structure.
pub struct Tree<T> {
    data: Vec<T>, offsets: Vec<usize>, children: Vec<usize>,
}

impl<T> Tree<T> {
    /// Constructs a tree based on the ordering imposed by a traversal.
    ///
    /// In the resulting tree, node data will be laid out in memory in the same
    /// order in which they are visited by the traversal imposed by `queue`.
    pub fn from_traversal<Q, I>(mut queue: Q, data: T, children: I) -> Self
        where Q: Queue<(usize, usize, T, I)>, I: Iterator<Item=(T, I)> {
            let mut tree = Tree { data: Vec::new(), offsets: Vec::new(), children: Vec::new(), };
            tree.data.push(data);
            tree.offsets.push(0);
            {
                let mut child_index = 0usize;
                for (data, children) in children {
                    queue.unshift((0, child_index, data, children));
                    child_index += 1;
                    tree.children.push(0);
                }
            }
            loop {
                match queue.shift() {
                    None => return tree,
                    Some((parent_index, index, data, children)) => {
                        tree.data.push(data);
                        tree.offsets.push(tree.children.len());
                        tree.children[tree.offsets[parent_index] + index] = index;
                        let mut child_index = 0usize;
                        for (data, children) in children {
                            queue.unshift((index, child_index, data, children));
                            child_index += 1;
                            tree.children.push(0);
                        }
                    }
                }
            }
        }

    /// Constructs a new tree with no children and the given data.
    pub fn leaf(data: T) -> Self {
        Tree { data: vec![data], offsets: vec![0], children: Vec::new(), }
    }

    /// Returns the number of nodes in this tree.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Returns a borrowed view of the nodes, in the order in which they are
    /// stored.
    pub fn nodes(&self) -> &[T] {
        &self.data
    }

    /// Returns a borrowed mutable view of the nodes, in the order in which they
    /// are stored.
    pub fn nodes_mut(&mut self) -> &mut [T] {
        &mut self.data
    }

    fn child_count(&self, index: usize) -> usize {
        match index.checked_add(1) {
            None =>
                panic!["numerical overflow in computing child count"],
            Some(x) if x > self.size() =>
                panic!["no such child {} (only {} nodes in tree)", index, self.size()],
            Some(x) if x == self.size() =>
                self.size() - self.offsets[index],
            Some(x) =>
                self.offsets[x] - self.offsets[index],
        }
    }

    fn child_of(&self, parent: usize, index: usize) -> usize {
        assert![parent < self.size()];
        match self.offsets[parent].checked_add(index) {
            Some(x) => self.children[x],
            None => panic!["numerical overflow in computing child offset"],
        }
    }
}

#[derive(Clone, Copy)]
enum TreePosition {
    Root,
    Nonroot(TreePositionData),
}

#[derive(Clone, Copy)]
struct TreePositionData {
    // Index into tree array.
    tree_index: usize,
    // Index in sequence of children under parent.
    parent_index: usize,
}

pub struct TreeView<'a, T: 'a> {
    tree: &'a Tree<T>, path: Vec<TreePosition>,
}

impl<'a, T: 'a> TreeView<'a, T> {
    fn here(&self) -> TreePosition {
        *self.path.last().unwrap()
    }    
}

impl<'a, T: 'a> Clone for TreeView<'a, T> {
    fn clone(&self) -> Self {
        TreeView { tree: self.tree, path: self.path.clone(), }
    }
}

impl<'a, T: 'a> Borrow<T> for TreeView<'a, T> {
    fn borrow(&self) -> &T {
        match self.here() {
            TreePosition::Root => &self.tree.data[0],
            TreePosition::Nonroot(data) => &self.tree.data[data.tree_index],
        }
    }
}

impl<'a, T: 'a> Nav for TreeView<'a, T> {
    fn seek_sibling(&mut self, offset: isize) {
        let new_index = match self.path.pop() {
            None => unreachable!(),
            Some(TreePosition::Root) => SiblingIndex::Root,
            Some(TreePosition::Nonroot(data)) => match self.here() {
                TreePosition::Root =>
                    SiblingIndex::compute(self.tree.child_count(0), 0, offset),
                TreePosition::Nonroot(parent_data) =>
                    SiblingIndex::compute(self.tree.child_count(parent_data.tree_index),
                                          data.parent_index,
                                          offset),
            },
        }.unwrap();
        let tree_index = match self.here() {
            TreePosition::Root =>
                self.tree.child_of(0, new_index),
            TreePosition::Nonroot(data) =>
                self.tree.child_of(data.tree_index, new_index),
        };
        self.path.push(TreePosition::Nonroot(
            TreePositionData { tree_index: tree_index, parent_index: new_index, }));
    }

    fn seek_child(&mut self, index: usize) {
        let new_index =
            ChildIndex::compute(self.child_count(), index).unwrap();
        let tree_index = match self.here() {
            TreePosition::Root => self.tree.child_of(0, new_index),
            TreePosition::Nonroot(data) => self.tree.child_of(data.tree_index, new_index),
        };
        self.path.push(TreePosition::Nonroot(
            TreePositionData { tree_index: tree_index, parent_index: new_index, }));
    }

    fn child_count(&self) -> usize {
        match self.here() {
            TreePosition::Root => self.tree.child_count(0),
            TreePosition::Nonroot(data) => self.tree.child_count(data.tree_index),
        }
    }

    fn at_root(&self) -> bool {
        self.path.len() == 1
    }

    fn to_parent(&mut self) {
        assert![self.path.len() <= 1, "Already at root"];
        self.path.pop();
    }

    fn to_root(&mut self) {
        self.path.clear();
        self.path.push(TreePosition::Root);
    }
}

pub struct TreeViewMut<'a, T: 'a> {
    tree: &'a mut Tree<T>,
    path: Vec<TreePosition>,
}

impl<'a, T> TreeViewMut<'a, T> {
    fn here(&self) -> TreePosition {
        *self.path.last().unwrap()
    }
}

impl<'a, T: 'a> Borrow<T> for TreeViewMut<'a, T> {
    fn borrow(&self) -> &T {
        match self.here() {
            TreePosition::Root => &self.tree.data[0],
            TreePosition::Nonroot(data) => &self.tree.data[data.tree_index],
        }
    }
}

impl<'a, T: 'a> BorrowMut<T> for TreeViewMut<'a, T> {
    fn borrow_mut(&mut self) -> &mut T {
        match self.here() {
            TreePosition::Root => &mut self.tree.data[0],
            TreePosition::Nonroot(data) => &mut self.tree.data[data.tree_index],
        }
    }
}

impl<'a, T: 'a> Nav for TreeViewMut<'a, T> {
    fn seek_sibling(&mut self, offset: isize) {
        let new_index = match self.path.pop() {
            None => unreachable!(),
            Some(TreePosition::Root) => SiblingIndex::Root,
            Some(TreePosition::Nonroot(data)) => match self.here() {
                TreePosition::Root =>
                    SiblingIndex::compute(self.tree.child_count(0), 0, offset),
                TreePosition::Nonroot(parent_data) =>
                    SiblingIndex::compute(self.tree.child_count(parent_data.tree_index),
                                          data.parent_index,
                                          offset),
            },
        }.unwrap();
        let tree_index = match self.here() {
            TreePosition::Root =>
                self.tree.child_of(0, new_index),
            TreePosition::Nonroot(data) =>
                self.tree.child_of(data.tree_index, new_index),
        };
        self.path.push(TreePosition::Nonroot(
            TreePositionData { tree_index: tree_index, parent_index: new_index, }));
    }

    fn seek_child(&mut self, index: usize) {
        let new_index =
            ChildIndex::compute(self.child_count(), index).unwrap();
        let tree_index = match self.here() {
            TreePosition::Root => self.tree.child_of(0, new_index),
            TreePosition::Nonroot(data) => self.tree.child_of(data.tree_index, new_index),
        };
        self.path.push(TreePosition::Nonroot(
            TreePositionData { tree_index: tree_index, parent_index: new_index, }));
    }

    fn child_count(&self) -> usize {
        match self.here() {
            TreePosition::Root => self.tree.child_count(0),
            TreePosition::Nonroot(data) => self.tree.child_count(data.tree_index),
        }
    }

    fn at_root(&self) -> bool {
        self.path.len() == 1
    }

    fn to_parent(&mut self) {
        assert![self.path.len() <= 1, "already at root"];
        self.path.pop();
    }

    fn to_root(&mut self) {
        self.path.clear();
        self.path.push(TreePosition::Root);
    }
}

#[cfg(test)]
mod tests {
    use ::fixed::Tree;
    
    #[test]
    fn basic() {
        Tree { data: vec![0], offsets: vec![0], children: vec![], };
    }
}
