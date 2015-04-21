use ::{Guard, Nav};
use ::traversal::Queue;
use ::util::{ChildIndex, SiblingIndex};

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
    /// In the resulting tree, nodes will be laid out in memory in the same
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

    pub fn size(&self) -> usize {
        self.data.len()
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

pub struct DataGuard<'a, T: 'a> {
    tree: &'a Tree<T>, index: usize,
}

impl<'a, T: 'a> Guard<'a, T> for DataGuard<'a, T> {
    fn super_deref<'s>(&'s self) -> &'a T {
        &self.tree.data[self.index]
    }
}

pub struct Navigator<'a, T: 'a> {
    tree: &'a Tree<T>, path: Vec<usize>,
}

impl<'a, T: 'a> Navigator<'a, T> {
    fn index(&self) -> usize {
        self.path[self.path.len() - 1]
    }
}

impl<'a, T: 'a> Clone for Navigator<'a, T> {
    fn clone(&self) -> Self {
        Navigator { tree: self.tree, path: self.path.clone(), }
    }
}

impl<'a, T: 'a> Nav<'a> for Navigator<'a, T> {
    type Data = T;
    type DataGuard = DataGuard<'a, T>;

    fn seek_sibling(&mut self, offset: isize) {
        let new_index =
            if self.path.len() < 1 {
                SiblingIndex::Root
            } else {
                let parent = self.path[self.path.len() - 1];
                SiblingIndex::compute(self.tree.child_count(parent),
                                      parent,
                                      offset)
            }.unwrap();
        let offset_index = match self.path.pop() {
            Some(parent) =>
                self.tree.child_of(parent, new_index),
            None =>
                panic!["tree corruption"],
        };
        self.path.push(offset_index);
    }

    fn seek_child(&mut self, index: usize) {
        let new_index =
            ChildIndex::compute(self.child_count(), index).unwrap();
        let offset = self.tree.offsets[self.index()];
        let child = self.tree.children[offset + new_index];
        self.path.push(child);
    }

    fn child_count(&self) -> usize {
        self.tree.child_count(self.index())
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
        self.path.push(0);
    }

    fn data(&self) -> DataGuard<'a, T> {
        DataGuard { tree: self.tree, index: self.index(), }
    }
}
