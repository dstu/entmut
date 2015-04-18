use ::{Guard, Nav};
use ::util::{ChildIndex, SiblingIndex};

use std::clone::Clone;
use std::iter::Iterator;

/// Single-ownership trees wherein a parent owns its children.
///
/// This tree structure keeps its children in a heap-allocated array, so
/// appending children is a cheap operation. References into the tree cannot be
/// retained when modifying it, however, and subtrees cannot be shared between
/// parents.
pub struct Tree<T> {
    data: T, children: Vec<Tree<T>>,
}

impl<T> Tree<T> {
    // TODO: loop instead of recurring to avoid blowing the stack.
    pub fn new<I: Iterator<Item=(T, I)>>(data: T, children: I) -> Self {
        let mut t = Tree { data: data, children: vec![], };
        for (child_data, child_children) in children {
            t.children.push(Tree::new(child_data, child_children));
        }
        t.children.shrink_to_fit();
        return t;
    }

    pub fn nav<'s>(&'s self) -> Navigator<'s, T> {
        Navigator::new(self)
    }
}

pub struct DataGuard<'a, T: 'a> {
    tree: &'a Tree<T>,
}

impl<'a, T: 'a> Guard<'a, T> for DataGuard<'a, T> {
    fn super_deref<'s>(&'s self) -> &'a T {
        &self.tree.data
    }
}

pub struct Navigator<'a, T: 'a> {
    here: &'a Tree<T>,
    path: Vec<(&'a Tree<T>, usize)>,
}

impl<'a, T: 'a> Navigator<'a, T> {
    fn new(tree: &'a Tree<T>) -> Self {
        Navigator { here: tree, path: Vec::new(), }
    }
}

impl<'a, T: 'a> Clone for Navigator<'a, T> {
    fn clone(&self) -> Self {
        Navigator { here: self.here, path: self.path.clone(), }
    }
}

impl<'a, T: 'a> Nav<'a> for Navigator<'a, T> {
    type Data = T;
    type DataGuard = DataGuard<'a, T>;

    fn seek_sibling(&mut self, offset: isize) {
        let new_index = {
            if self.at_root() {
                SiblingIndex::Root
            } else {
                let (parent, here_index) = self.path[self.path.len() - 1];
                SiblingIndex::compute(parent.children.len(),
                                      here_index,
                                      offset)
            }
        }.unwrap();
        let (parent, _) = self.path.pop().unwrap();
        self.path.push((parent, new_index));
        self.here = &parent.children[new_index];
    }

    fn seek_child(&mut self, index: usize) {
        let new_index =
            ChildIndex::compute(self.here.children.len(), index).unwrap();
        self.path.push((self.here, new_index));
        self.here = &self.here.children[new_index];
    }

    fn child_count(&self) -> usize {
        self.here.children.len()
    }

    fn at_root(&self) -> bool {
        self.path.is_empty()
    }

    fn to_parent(&mut self) {
        let (parent, _) = self.path.pop().expect("already at root");
        self.here = parent;
    }

    fn to_root(&mut self) {
        if ! self.at_root() {
            let (parent, _) = self.path[0];
            self.here = parent;
            self.path.clear();
        }
    }

    fn data(&self) -> DataGuard<'a, T> {
        DataGuard { tree: self.here, }
    }
}
