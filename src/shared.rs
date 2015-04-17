use ::{Guard, Nav};
use ::util::{ChildIndex, SiblingIndex};

use std::cell::RefCell;
use std::rc::Rc;

struct TreeInternal<T> {
    data: RefCell<T>, children: RefCell<Vec<Tree<T>>>,
}

pub struct Tree<T> {
    internal: Rc<TreeInternal<T>>,
}

pub struct DataGuard<'a, T: 'a> {
    cell: &'a RefCell<T>,
}

impl<'a, T: 'a> Guard<'a, RefCell<T>> for DataGuard<'a, T> {
    fn super_deref<'s>(&'s self) -> &'a RefCell<T> {
        self.cell
    }
}

pub struct Navigator<'a, T: 'a> {
    here: &'a Tree<T>, path: Vec<(&'a Tree<T>, usize)>,
}

impl<'a, T: 'a> Nav<'a> for Navigator<'a, T> {
    type Data = RefCell<T>;
    type DataGuard = DataGuard<'a, T>;

    fn seek_sibling(&mut self, offset: isize) {
        let new_index = {
            if self.at_root() {
                SiblingIndex::Root
            } else {
                let (parent, here_index) = self.path[self.path.len() - 1];
                SiblingIndex::compute(parent.internal.children.borrow().len(),
                                      here_index,
                                      offset)
            }
        }.unwrap();
        let (parent, _) = self.path.pop().unwrap();
        self.path.push((parent, new_index));
        self.here = &parent.internal.children.borrow()[new_index];
    }

    fn seek_child(&mut self, index: usize) {
        let new_index =
            ChildIndex::compute(self.here.internal.children.borrow().len(), index).unwrap();
        self.path.push((self.here, new_index));
        self.here = &self.here.internal.children.borrow()[new_index];
    }

    fn child_count(&self) -> usize {
        self.here.internal.children.borrow().len()
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
            let (root, _) = self.path[0];
            self.here = root;
            self.path.clear();
        }
    }

    fn data(&self) -> DataGuard<'a, T> {
        DataGuard { cell: &self.here.internal.data, }
    }
}
