use ::{Guard, Nav};
use ::util::{ChildIndex, SiblingIndex};

use std::cell::{Ref, RefCell};
use std::mem;
use std::rc::Rc;

struct TreeInternal<T> {
    data: T, children: RefCell<Vec<Tree<T>>>,
}

pub struct Tree<T> {
    internal: Rc<TreeInternal<T>>,
}

pub struct DataGuard<T> {
    tree: Tree<T>,
}

impl<'a, T: 'a> Guard<'a, T> for DataGuard<T> {
    fn super_deref<'s>(&'s self) -> &'a T {
        unsafe {
            mem::transmute(&self.tree.internal.data)
        }
    }
}

pub struct Navigator<'a, T: 'a> {
    root: &'a Tree<T>,
    path: Vec<(Ref<'a, Vec<Tree<T>>>, usize)>,
}

impl<'a, T: 'a> Navigator<'a, T> {
    fn here<'s>(&'s self) -> &'s Tree<T> {
        match self.path.last() {
            None => self.root,
            Some(&(ref siblings, ref index)) => &siblings[*index],
        }
    }
}

impl<'a, T: 'a> Nav<'a> for Navigator<'a, T> {
    type Data = T;
    type DataGuard = DataGuard<T>;

    fn seek_sibling(&mut self, offset: isize) {
        let new_index = 
            match self.path.last() {
                None => SiblingIndex::Root,
                Some(&(ref siblings, ref index)) =>
                    SiblingIndex::compute(siblings.len(), *index, offset),
            }.unwrap();
        let (siblings, _) = self.path.pop().unwrap();
        self.path.push((siblings, new_index));
    }

    fn seek_child(&mut self, index: usize) {
        let new_index =
            ChildIndex::compute(self.here().internal.children.borrow().len(), index).unwrap();
        let children = unsafe {
            mem::transmute(self.here().internal.children.borrow())
        };
        self.path.push((children, new_index));
    }

    fn child_count(&self) -> usize {
        self.here().internal.children.borrow().len()
    }

    fn at_root(&self) -> bool {
        self.path.is_empty()
    }

    fn to_parent(&mut self) {
        self.path.pop().expect("already at root");
    }

    fn to_root(&mut self) {
        self.path.clear();
    }

    fn data(&self) -> DataGuard<T> {
        DataGuard { tree: Tree { internal: self.here().internal.clone(), } }
    }
}
