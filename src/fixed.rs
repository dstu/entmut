use ::{Guard, Nav};

use std::borrow::Borrow;
use std::ops::Deref;

pub struct Tree<T> {
    data: Vec<T>, offsets: Vec<usize>, children: Vec<usize>,
}

impl<T> Tree<T> {
    pub fn nodes(&self) -> usize {
        self.data.len()
    }

    fn child_count(&self, index: usize) -> usize {
        // TODO make arithmetic safe.
        if index + 1 < self.nodes() {
            self.offsets[index + 1] - self.offsets[index]
        } else if index + 1 == self.nodes() {
            self.nodes() - self.offsets[index]
        } else {
            panic!["No such child {} (only {} nodes in tree)",
                   index, self.nodes()]
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

impl<'a, T: 'a> Nav<'a> for Navigator<'a, T> {
    type Data = T;
    type DataGuard = DataGuard<'a, T>;

    fn seek_sibling(&mut self, offset: isize) {
        // TODO
    }

    fn seek_child(&mut self, index: usize) {
        // TODO
    }

    fn child_count(&self) -> usize {
        self.tree.child_count(self.index())
    }

    fn is_root(&self) -> bool {
        self.path.len() == 1
    }

    fn to_parent(&mut self) {
        assert![self.path.len() <= 1, "Already at root"];
        self.path.pop();
    }

    fn data(&self) -> DataGuard<'a, T> {
        DataGuard { tree: self.tree, index: self.index(), }
    }
}
