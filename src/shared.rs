use ::{Guard, Nav};

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
    here: &'a Tree<T>, path: Vec<&'a Tree<T>>,
}

impl<'a, T: 'a> Nav<'a> for Navigator<'a, T> {
    type Data = RefCell<T>;
    type DataGuard = DataGuard<'a, T>;

    fn seek_sibling(&mut self, offset: isize) {
        // TODO
    }

    fn seek_child(&mut self, index: usize) {
        // TODO
    }

    fn child_count(&self) -> usize {
        self.here.internal.children.borrow().len()
    }

    fn is_root(&self) -> bool {
        self.path.is_empty()
    }

    fn to_parent(&mut self) {
        self.here = self.path.pop().expect("Already at root");
    }

    fn data(&self) -> DataGuard<'a, T> {
        DataGuard { cell: &self.here.internal.data, }
    }
}
