use ::{Guard, Nav};
use ::util::{ChildIndex, SiblingIndex};

use std::cell::{Ref, RefCell};
use std::clone::Clone;
use std::mem;
use std::rc::Rc;

/// Heap-allocated, reference-counted trees that can be shared freely.

struct TreeInternal<T> {
    data: T, children: RefCell<Vec<Tree<T>>>,
}

/// Reference to a heap-allocated tree.
/// 
/// This tree structure has the same characteristics as
/// [owned::Tree](../owned/struct.Tree.html), except that a parent does not own
/// its children. Internally, this is achieved by storing trees in `std::rc::Rc`
/// wrappers. As a result, this type can be cloned and shared as the child of
/// multiple parents. This may be useful for saving memory.
pub struct Tree<T> {
    internal: Rc<TreeInternal<T>>,
}

impl<T> Tree<T> {
    pub fn new(data: T, children: Vec<Tree<T>>) -> Self {
        Tree { internal: Rc::new(TreeInternal { data: data, children: RefCell::new(children), }), }
    }

    pub fn leaf(data: T) -> Self {
        Tree { internal: Rc::new(TreeInternal { data: data, children: RefCell::new(Vec::new()), }), }
    }

    pub fn push_child(&mut self, child: Tree<T>) {
        self.internal.children.borrow_mut().push(child);
    }

    pub fn remove_child(&mut self, index: usize) {
        assert![index < self.internal.children.borrow().len(),
                "cannot remove child at index {} (only {} children)", index, self.internal.children.borrow().len()];
        self.internal.children.borrow_mut().remove(index);
    }

    pub fn insert_child(&mut self, index: usize, child: Tree<T>) {
        self.internal.children.borrow_mut().insert(index, child);
    }

    pub fn nav<'s>(&'s self) -> Navigator<'s, T> {
        Navigator::new(self)
    }
}

/// Creates a new reference to this tree, such that modifying the reference also
/// modifies the original tree.
impl<T> Clone for Tree<T> {
    fn clone(&self) -> Self {
        Tree { internal: self.internal.clone(), }
    }
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
    fn new(root: &'a Tree<T>) -> Self {
        Navigator { root: root, path: Vec::new(), }
    }

    fn here<'s>(&'s self) -> &'s Tree<T> {
        match self.path.last() {
            None => self.root,
            Some(&(ref siblings, ref index)) => &siblings[*index],
        }
    }
}

/// Due to the internal representation of the path back from the tree root, this
/// `Clone` implementation retraces the path from the root. This may be less
/// efficient than is desirable.
impl<'a, T: 'a> Clone for Navigator<'a, T> {
    fn clone(&self) -> Self {
        // We can't clone self.path directly, so we rebuild it by hand.
        let mut new_nav = Navigator { root: self.root, path: Vec::new(), };
        new_nav.path.reserve(self.path.len());
        for &(_, index) in &self.path {
            new_nav.seek_child(index);
        }
        return new_nav;
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
        DataGuard { tree: self.here().clone(), }
    }
}
