use ::{Guard, Nav};

use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::usize;

struct ChildNode<T> {
    tree: Tree<T>,
    prev: Option<Rc<ChildNode<T>>>,
    next: Option<Rc<ChildNode<T>>>,
}

struct TreeInternal<T> {
    data: T, first_child: Option<Rc<RefCell<ChildNode<T>>>>,
}

pub struct Tree<T> {
    internal: Rc<TreeInternal<T>>,
}

pub struct DataGuard<'a, T: 'a> {
    internal: &'a TreeInternal<T>,
}

impl<'a, T: 'a> Guard<'a, T> for DataGuard<'a, T> {
    fn super_deref<'s>(&'s self) -> &'a T {
        &self.internal.data
    }
}

pub struct Navigator<'a, T: 'a> {
    here: &'a Tree<T>, path: Vec<&'a Tree<T>>,
}

impl<'a, T: 'a> Nav<'a> for Navigator<'a, T> {
    type Data = T;
    type DataGuard = DataGuard<'a, T>;

    fn seek_sibling(&mut self, offset: isize) {
        let parent = self.path.pop().expect("Root has no siblings");
        let mut node = parent.internal.first_child;
        loop {
            match node {
                None => panic!["No such sibling"],
                Some(s) if offset == 0 => {
                    self.here = &s.tree;
                    break;
                },
                Some(s) if offset < 0 => {
                    node = s.borrow().prev;
                    offset += 1;
                },
                Some(s) if offset > 0 => {
                    node = s.borrow().next;
                    offset -= 1;
                },
            }
        }
    }

    fn seek_child(&mut self, index: usize) {
        let mut node = &self.here.internal.borrow().first_child;
        loop {
            match node {
                &None => panic!["No such child"],
                &Some(s) if index == 0 => {
                    self.path.push(Tree { internal: self.here.internal.clone() });
                    self.here = Tree { internal: s.borrow().tree.internal.clone() };
                },
                &Some(s) if index > 0 => {
                    node = &s.borrow().next;
                    index -= 1;
                },
            }
        }
    }

    fn at_root(&self) -> bool {
        self.path.is_empty()
    }

    fn to_parent(&mut self) {
        self.here = self.path.pop().expect("Already at root");
    }

    fn to_root(&mut self) {
        if ! self.at_root() {
            self.here = self.path[0];
            self.path.clear();
        }
    }

    fn data(&self) -> DataGuard<'a, T> {
        DataGuard { data: &self.here.internal, }
    }

    fn child_count(&self) -> usize {
        let mut node = &self.here.internal.borrow().first_child;
        for i in 0us.. {
            match node {
                &None => return i,
                &Some(s) => {
                    node = &s.borrow().next;
                    i += 1;
                },
            }
        }
        return usize::MAX;
    }
}
