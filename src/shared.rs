use ::{Guard, Editor, Nav, View};
use ::util::{ChildIndex, SiblingIndex};

use std::cell::{Ref, RefCell, RefMut};
use std::clone::Clone;
use std::mem;
use std::rc::Rc;

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

    pub fn view<'s>(&'s self) -> TreeView<'s, T> {
        TreeView::new(self)
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

pub struct TreeView<'a, T: 'a> {
    root: &'a Tree<T>,
    path: Vec<(Ref<'a, Vec<Tree<T>>>, usize)>,
}

impl<'a, T: 'a> TreeView<'a, T> {
    fn new(root: &'a Tree<T>) -> Self {
        TreeView { root: root, path: Vec::new(), }
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
impl<'a, T: 'a> Clone for TreeView<'a, T> {
    fn clone(&self) -> Self {
        // We can't clone self.path directly, so we rebuild it by hand.
        let mut new_nav = TreeView { root: self.root, path: Vec::new(), };
        new_nav.path.reserve(self.path.len());
        for &(_, index) in &self.path {
            new_nav.seek_child(index);
        }
        return new_nav;
    }
}

impl<'a, T: 'a> View<'a> for TreeView<'a, T> {
    type Data = T;
    type DataGuard = DataGuard<T>;
    fn data(&self) -> DataGuard<T> {
        DataGuard { tree: self.here().clone(), }
    }
}

impl<'a, T: 'a> Nav for TreeView<'a, T> {
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
}

pub struct TreeEditor<'a, T: 'a> {
    root: &'a mut Tree<T>,
    path: Vec<(RefMut<'a, Vec<Tree<T>>>, usize)>,
}

impl<'a, T: 'a> TreeEditor<'a, T> {
    fn here(&self) -> &Tree<T> {
        if self.path.is_empty() {
            self.root
        } else {
            let &(ref parent, index) = &self.path[self.path.len() - 1];
            &parent[index]
        }
    }

    fn here_mut(&mut self) -> &mut Tree<T> {
        if self.path.is_empty() {
            self.root
        } else {
            let path_index = self.path.len() - 1;
            let &mut (ref mut parent, index) = &mut self.path[path_index];
            &mut parent[index]
        }
    }
}

impl<'a, T: 'a> Nav for TreeEditor<'a, T> {
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
}

impl<'a, T: 'a> Editor for TreeEditor<'a, T> {
    type Data = T;
    type Tree = Tree<T>;

    fn push_leaf(&mut self, data: T) {
        self.push_child(Tree::leaf(data));
    }

    fn push_child(&mut self, child: Tree<T>) {
        match self.path.pop() {
            None => {
                self.root.internal.children.borrow_mut().push(child);
                let last_child_index = self.child_count() - 1;
                self.seek_child(last_child_index);
            },
            Some((parent_children, here_index)) => {
                let child_index = {
                    let mut here_children =
                        parent_children[here_index].internal.children.borrow_mut();
                    here_children.push(child);
                    here_children.len() - 1
                };
                self.path.push((parent_children, here_index));
                let last_path_index = self.path.len() - 1;
                let children: RefMut<'a, Vec<Tree<T>>> = unsafe {
                    mem::transmute(self.path[last_path_index].0[here_index].internal.children.borrow_mut())
                };
                self.path.push((children, child_index));
            },
        }
    }

    fn insert_leaf(&mut self, index: usize, data: T) {
        self.insert_child(index, Tree::leaf(data));
    }

    fn insert_child(&mut self, index: usize, child: Tree<T>) {
        match self.path.pop() {
            None => {
                let mut children: RefMut<'a, Vec<Tree<T>>> = unsafe {
                    mem::transmute(self.root.internal.children.borrow_mut())
                };
                let new_index = ChildIndex::compute(children.len(), index).unwrap();
                children.insert(new_index, child);
                self.path.push((children, index));
            },
            Some((parent_children, here_index)) => {
                let mut children: RefMut<'a, Vec<Tree<T>>> = unsafe {
                    mem::transmute(parent_children[here_index].internal.children.borrow_mut())
                };
                let new_index = ChildIndex::compute(children.len(), index).unwrap();
                children.insert(new_index, child);
                self.path.push((children, new_index));
            },
        }
    }

    fn insert_sibling_leaf(&mut self, offset: isize, data: T) {
        self.insert_sibling(offset, Tree::leaf(data));
    }

    fn insert_sibling(&mut self, offset: isize, sibling: Tree<T>) {
        let new_index =
            match self.path.last() {
                None => SiblingIndex::Root,
                Some(&(ref siblings, ref index)) =>
                    SiblingIndex::compute(siblings.len(), *index, offset),
            }.unwrap();
        let (mut siblings, _) = self.path.pop().unwrap();
        siblings.insert(new_index, sibling);
        self.path.push((siblings, new_index));
    }

    fn remove(&mut self) -> Tree<T> {
        let (mut parent_children, mut here_index) =
            self.path.pop().expect("already at root");
        if parent_children.len() != 0 {
            let removed = parent_children.remove(here_index);
            // We will wind up pointing at a sibling.
            if here_index < parent_children.len() - 1 {
                // We can keep pointing at the same index in parent.
                self.path.push((parent_children, here_index));
            } else {
                // At rightmost child, so we bump the index one to the left.
                here_index -= 1;
                self.path.push((parent_children, here_index));
            }
            removed
        } else {
            // We will wind up pointing to parent.
            parent_children.remove(0)
        }
    }

    fn remove_child(&mut self, index: usize) -> Tree<T> {
        match self.path.pop() {
            None => {
                // At root.
                self.root.internal.children.borrow_mut().remove(index)
            },
            Some((parent_children, here_index)) => {
                let mut children =
                    parent_children[here_index].internal.children.borrow_mut();
                children.remove(here_index)
            },
        }
    }

    fn remove_sibling(&mut self, offset: isize) -> Tree<T> {
        let index = {
            match self.path.last() {
                None => SiblingIndex::Root,
                Some(&(ref parent_children, here_index)) => 
                    SiblingIndex::compute(
                        parent_children.len(), here_index, offset),
            }
        }.unwrap();
        let (mut parent_children, here_index) = self.path.pop().unwrap();
        let removed = parent_children.remove(index);
        let new_index =
            if index > here_index {
                here_index
            } else {
                here_index - 1
            };
        self.path.push((parent_children, new_index));
        removed
    }

    fn swap(&mut self, other: &mut Tree<T>) {
        match self.path.last_mut() {
            None => mem::swap(self.root, other),
            Some(&mut (ref mut parent_children, here_index)) =>
                mem::swap(&mut parent_children[here_index], other),
        }
    }

    fn swap_children(&mut self, index_a: usize, index_b: usize) {
        if index_a == index_b {
            return;
        }
        self.here_mut().internal.children.borrow_mut().swap(index_a, index_b);
    }

    fn swap_siblings(&mut self, offset_a: isize, offset_b: isize) {
        if offset_a == offset_b {
            return;
        }
        let (index_a, index_b) = {
            let (a, b) = {
                match self.path.last() {
                    None => (SiblingIndex::Root, SiblingIndex::Root),
                    Some(&(ref parent_children, here_index)) =>
                        (SiblingIndex::compute(parent_children.len(), here_index, offset_a),
                         SiblingIndex::compute(parent_children.len(), here_index, offset_b)),
                }
            };
            (a.unwrap(), b.unwrap())
        };
        let (mut parent_children, mut here_index) = self.path.pop().unwrap();
        parent_children.swap(index_a, index_b);
        if here_index == index_a {
            here_index = index_b;
        } else if here_index == index_b {
            here_index = index_a;
        }
        self.path.push((parent_children, here_index));
    }
}
