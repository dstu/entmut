use ::{Editor, Nav};
use ::util::{ChildIndex, SiblingIndex};

use std::borrow::{Borrow, BorrowMut};
use std::clone::Clone;
use std::fmt;
use std::iter::Iterator;
use std::ptr;

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
    pub fn new(data: T, children: Vec<Tree<T>>) -> Self {
        Tree { data: data, children: children, }
    }

    pub fn leaf(data: T) -> Self {
        Tree { data: data, children: Vec::new(), }
    }

    pub fn push_child(&mut self, child: Tree<T>) {
        self.children.push(child);
    }

    pub fn remove_child(&mut self, index: usize) {
        assert![index < self.children.len(),
                "cannot remove child at index {} (only {} children)", index, self.children.len()];
        self.children.remove(index);
    }

    pub fn insert_child(&mut self, index: usize, child: Tree<T>) {
        self.children.insert(index, child);
    }

    pub fn into_parts(self) -> (T, Vec<Tree<T>>) {
        (self.data, self.children)
    }

    pub fn view<'s>(&'s self) -> TreeView<'s, T> {
        TreeView::new(self)
    }

    pub fn view_mut<'s>(&'s mut self) -> TreeViewMut<'s, T> {
        TreeViewMut::new(self)
    }
}

impl<T: PartialEq> PartialEq<Tree<T>> for Tree<T> {
    fn eq(&self, other: &Tree<T>) -> bool {
        let mut x_stack = vec![self];
        let mut y_stack = vec![other];
        loop {
            match (x_stack.pop(), y_stack.pop()) {
                (None, None) => return true,
                (Some(x), Some(y)) if x.data == y.data => {
                    for child in x.children.iter() {
                        x_stack.push(child);
                    }
                    for child in y.children.iter() {
                        y_stack.push(child);
                    }
                },
                _ => return false,
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Tree<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        enum PathElement<'a, T: 'a> {
            Down(&'a Tree<T>),
            Up,
        }
        try![f.write_str("(")];
        try![self.data.fmt(f)];
        let mut stack = vec![];
        for child in self.children.iter().rev() {
            stack.push(PathElement::Up);
            stack.push(PathElement::Down(child));
        }
        loop {
            match stack.pop() {
                Some(PathElement::Down(t)) => {
                    try![f.write_str(" (")];
                    try![t.data.fmt(f)];
                    for child in t.children.iter().rev() {
                        stack.push(PathElement::Up);
                        stack.push(PathElement::Down(child));
                    }
                },
                Some(PathElement::Up) => try![f.write_str(")")],
                None => {
                    try![f.write_str(")")];
                    return Result::Ok(())
                },
            }
        }
    }
}

pub struct TreeView<'a, T: 'a> {
    here: &'a Tree<T>,
    path: Vec<(&'a Tree<T>, usize)>,
}

impl<'a, T: 'a> TreeView<'a, T> {
    fn new(tree: &'a Tree<T>) -> Self {
        TreeView { here: tree, path: Vec::new(), }
    }
}

impl<'a, T: 'a> Clone for TreeView<'a, T> {
    fn clone(&self) -> Self {
        TreeView { here: self.here, path: self.path.clone(), }
    }
}

impl<'a, T: 'a> Borrow<T> for TreeView<'a, T> {
    fn borrow(&self) -> &T {
        &self.here.data
    }
}

impl<'a, T: 'a> Nav for TreeView<'a, T> {
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
        let new_index = ChildIndex::compute(self.child_count(), index).unwrap();
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
}

pub struct TreeViewMut<'a, T: 'a> {
    tree: &'a mut Tree<T>,
    here_ptr: *mut Tree<T>,
    path: Vec<(*mut Tree<T>, usize)>,
}

impl<'a, T: 'a> TreeViewMut<'a, T> {
    fn new(tree: &'a mut Tree<T>) -> Self {
        let tree_ptr: *mut Tree<T> = tree;
        TreeViewMut { tree: tree,
                      here_ptr: tree_ptr,
                      path: vec![], }
    }

    fn here(&self) -> &Tree<T> {
        unsafe { &*self.here_ptr }
    }

    fn here_mut(&mut self) -> &mut Tree<T> {
        unsafe { &mut *self.here_ptr }
    }
}

impl<'a, T: 'a> Borrow<T> for TreeViewMut<'a, T> {
    fn borrow(&self) -> &T {
        &self.here().data
    }
}

impl<'a, T: 'a> BorrowMut<T> for TreeViewMut<'a, T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.here_mut().data
    }
}

impl<'a, T: 'a> Nav for TreeViewMut<'a, T> {
    fn child_count(&self) -> usize {
        self.here().children.len()
    }

    fn at_root(&self) -> bool { self.path.is_empty() }

    fn seek_sibling(&mut self, offset: isize) {
        let new_index = {
            if self.at_root() {
                SiblingIndex::Root
            } else {
                let (parent_ptr, here_index) = self.path[self.path.len() - 1];
                let parent: &Tree<T> = unsafe { &*parent_ptr };
                SiblingIndex::compute(parent.children.len(),
                                      here_index,
                                      offset)
            }
        }.unwrap();
        let (parent_ptr, _) = self.path.pop().unwrap();
        self.path.push((parent_ptr, new_index));
        let parent: &mut Tree<T> = unsafe { &mut *parent_ptr };
        self.here_ptr = &mut parent.children[new_index];
    }

    fn seek_child(&mut self, index: usize) {
        let new_index = ChildIndex::compute(self.child_count(), index).unwrap();
        self.path.push((self.here_ptr, new_index));
        let t: &mut Tree<T> = unsafe { &mut *self.here_ptr };
        self.here_ptr = &mut t.children[new_index];
    }

    fn to_parent(&mut self) {
        let (parent_ptr, _) = self.path.pop().expect("already at root");
        self.here_ptr = parent_ptr;
    }

    fn to_root(&mut self) {
        if ! self.at_root() {
            self.path.clear();
            self.here_ptr = self.tree;
        }
    }
}

impl<'a, T: 'a> Editor for TreeViewMut<'a, T> {
    type Data = T;
    type Tree = Tree<T>;

    fn push_leaf(&mut self, data: T) {
        self.push_child(Tree::leaf(data));
    }

    fn push_child(&mut self, child: Tree<T>) {
        self.here_mut().children.push(child);
        let new_child_index = self.here().children.len() - 1;
        self.path.push((self.here_ptr, new_child_index));
        self.here_ptr = &mut self.here_mut().children[new_child_index];
    }

    fn insert_leaf(&mut self, index: usize, data: T) {
        self.insert_child(index, Tree::leaf(data));
    }
    
    fn insert_child(&mut self, index: usize, child: Tree<T>) {
        let new_index =
            ChildIndex::compute(self.here().children.len(), index).unwrap();
        self.here_mut().children.insert(new_index, child);
        self.path.push((self.here_ptr, new_index));
        self.here_ptr = &mut self.here_mut().children[new_index];
    }

    fn insert_sibling_leaf(&mut self, offset: isize, data: T) {
        self.insert_sibling(offset, Tree::leaf(data));
    }

    fn insert_sibling(&mut self, offset: isize, sibling: Tree<T>) {
        let new_index = {
            if self.at_root() {
                SiblingIndex::Root
            } else {
                let (parent_ptr, here_index) = self.path[self.path.len() - 1];
                let parent: &Tree<T> = unsafe { &*parent_ptr };
                SiblingIndex::compute(parent.children.len(),
                                      here_index,
                                      offset)
            }
        }.unwrap();
        let (parent_ptr, _) = self.path.pop().unwrap();
        let parent: &mut Tree<T> = unsafe { &mut *parent_ptr };
        parent.children.insert(new_index, sibling);
        self.path.push((parent_ptr, new_index));
        self.here_ptr = &mut parent.children[new_index];
    }

    fn remove(&mut self) -> Tree<T> {
        let (parent_ptr, mut here_index) =
            self.path.pop().expect("already at root");
        let parent: &mut Tree<T> = unsafe { &mut *parent_ptr };
        if parent.children.len() != 0 {
            let removed = parent.children.remove(here_index);
            // We will wind up pointing at a sibling.
            if here_index < parent.children.len() - 1 {
                // We can keep pointing at the same index in parent.
                self.path.push((parent_ptr, here_index));
                self.here_ptr = &mut parent.children[here_index];
            } else {
                // At rightmost child, so we bump the index one to the left.
                here_index -= 1;
                self.path.push((parent_ptr, here_index));
                self.here_ptr = &mut parent.children[here_index];
            }
            removed
        } else {
            // We will wind up pointing to parent.
            self.here_ptr = parent_ptr;
            parent.children.remove(0)
        }
    }

    fn remove_child(&mut self, index: usize) -> Tree<T> {
        let new_index = ChildIndex::compute(self.child_count(), index).unwrap();
        self.here_mut().children.remove(new_index)
    }

    fn remove_sibling(&mut self, offset: isize) -> Tree<T> {
        if offset == 0 {
            return self.remove();
        }
        let (parent_ptr, here_index) =
            self.path.pop().expect("already at root");
        let parent: &mut Tree<T> = unsafe { &mut *parent_ptr };
        let index =
            SiblingIndex::compute(
                parent.children.len(), here_index, offset).unwrap();
        let removed = parent.children.remove(index);
        let new_index =
            if index > here_index {
                here_index
            } else {
                here_index - 1
            };
        self.path.push((parent_ptr, new_index));
        self.here_ptr = &mut parent.children[new_index];
        removed
    }

    fn swap(&mut self, other: &mut Tree<T>) {
        unsafe { ptr::swap(self.here_ptr, other) };
    }

    fn swap_children(&mut self, index_a: usize, index_b: usize) {
        let new_index_a =
            ChildIndex::compute(self.child_count(), index_a).unwrap();
        let new_index_b =
            ChildIndex::compute(self.child_count(), index_b).unwrap();
        self.here_mut().children.swap(new_index_a, new_index_b);
    }

    fn swap_siblings(&mut self, offset_a: isize, offset_b: isize) {
        let index_a = {
            if self.at_root() {
                SiblingIndex::Root
            } else {
                let &(parent_ptr, here_index) = self.path.last().unwrap();
                let parent: &mut Tree<T> = unsafe { &mut *parent_ptr };
                SiblingIndex::compute(parent.children.len(), here_index, offset_a)
            }
        }.unwrap();
        let index_b = {
            let &(parent_ptr, here_index) = self.path.last().unwrap();
            let parent: &mut Tree<T> = unsafe { &mut *parent_ptr };
            SiblingIndex::compute(parent.children.len(), here_index, offset_b)
        }.unwrap();

        let &(parent_ptr, here_index) = self.path.last().unwrap();
        let parent: &mut Tree<T> = unsafe { &mut *parent_ptr };
        parent.children.swap(index_a, index_b);
        if here_index == index_a {
            self.here_ptr = &mut parent.children[index_a];
        } else if here_index == index_b {
            self.here_ptr = &mut parent.children[index_b];
        }
    }
}

#[macro_export]
macro_rules! owned_tree {
    ($data:expr) => ($crate::owned::Tree::leaf($data));
    ($data:expr, [$($first:tt)*] $(,[$($rest:tt)*])*) =>
        ($crate::owned::Tree::new($data, vec![owned_tree![$($first)*]
                                              $(,owned_tree![$($rest)*])*]));
}

#[cfg(test)]
mod test {
    use ::owned::Tree;

    #[test]
    fn eq_check() {
        assert_eq![Tree::leaf("a"), Tree::leaf("a")];
        assert![Tree::leaf("a") != Tree::leaf("b")];
        assert_eq![Tree::new("a", vec![Tree::leaf("b"), Tree::leaf("c")]),
                   Tree::new("a", vec![Tree::leaf("b"), Tree::leaf("c")])];
        assert![Tree::new("a", vec![Tree::leaf("c"), Tree::leaf("b")])
                != Tree::new("a", vec![Tree::leaf("b"), Tree::leaf("c")])];
    }

    #[test]
    fn leaf_literal() {
        assert_eq![owned_tree!["a"], Tree::leaf("a")];
    }

    #[test]
    fn other_literal() {
        assert_eq![owned_tree!["a", ["b"]],
                   Tree::new("a", vec![Tree::leaf("b")])];
        assert_eq![owned_tree!["a", ["b"], ["c"], ["d"]],
                   Tree::new("a", vec![Tree::leaf("b"),
                                       Tree::leaf("c"),
                                       Tree::leaf("d")])];
        assert_eq![owned_tree!["a", ["b", ["c", ["d"]]], ["e", ["f"]]],
                   Tree::new("a", vec![
                       Tree::new("b", vec![
                           Tree::new("c", vec![Tree::leaf("d")])]),
                       Tree::new("e", vec![Tree::leaf("f")])])];
    }

    #[test]
    fn push_child() {
        {
            let mut t = owned_tree!["a"];
            t.push_child(owned_tree!["b"]);
            assert_eq![t, owned_tree!["a", ["b"]]];
        }
        {
            let mut t = owned_tree!["a", ["b"]];
            t.push_child(owned_tree!["c"]);
            assert_eq![t, owned_tree!["a", ["b"], ["c"]]];
        }
        {
            let mut t = owned_tree!["a", ["b"]];
            t.children[0].push_child(owned_tree!["c"]);
            assert_eq![t, owned_tree!["a", ["b", ["c"]]]];
        }
    }

    #[test]
    #[should_panic]
    fn remove_child_panics_no_children() {
        owned_tree!["a"].remove_child(0);
    }

    #[test]
    #[should_panic]
    fn remove_child_panics_bad_index() {
        owned_tree!["a", ["b"], ["c"]].remove_child(2);
    }

    #[test]
    fn remove_child() {
        {
            let mut t = owned_tree!["a", ["b"]];
            t.remove_child(0);
            assert_eq![t, owned_tree!["a"]];
        }
        {
            let mut t = owned_tree!["a", ["b"], ["c"]];
            t.remove_child(0);
            assert_eq![t, owned_tree!["a", ["c"]]];
            t.remove_child(0);
            assert_eq![t, owned_tree!["a"]];
        }
        {
            let mut t = owned_tree!["a", ["b"], ["c"]];
            t.remove_child(1);
            assert_eq![t, owned_tree!["a", ["b"]]];
            t.remove_child(0);
            assert_eq![t, owned_tree!["a"]];
        }
    }

    #[test]
    #[should_panic]
    fn insert_child_panics_no_children() {
        owned_tree!["a"].insert_child(1, owned_tree!["b"]);
    }

    #[test]
    #[should_panic]
    fn insert_child_panics_bad_index() {
        owned_tree!["a", ["b"]].insert_child(2, owned_tree!["c"]);
    }

    #[test]
    fn insert_child_at_leaf() {
        let mut t = owned_tree!["a"];
        t.insert_child(0, owned_tree!["b"]);
        assert_eq![t, owned_tree!["a", ["b"]]];
    }

    #[test]
    fn insert_child_at_start() {
        let mut t = owned_tree!["a", ["b"], ["c", ["d"]], ["e"]];
        t.insert_child(0, owned_tree!["aa"]);
        assert_eq![t, owned_tree!["a", ["aa"], ["b"], ["c", ["d"]], ["e"]]];
    }

    #[test]
    fn insert_child_at_end() {
        let mut t = owned_tree!["a", ["b"], ["c", ["d"]], ["e"]];
        t.insert_child(3, owned_tree!["aa"]);
        assert_eq![t, owned_tree!["a", ["b"], ["c", ["d"]], ["e"], ["aa"]]];
    }

    #[test]
    fn insert_child_at_middle() {
        let mut t = owned_tree!["a", ["b"], ["c", ["d"]], ["e"]];
        t.insert_child(2, owned_tree!["aa"]);
        assert_eq![t, owned_tree!["a", ["b"], ["c", ["d"]], ["aa"], ["e"]]];
    }

    #[test]
    fn leaf_into_parts() {
        let t = owned_tree!["a"];
        let (data, children) = t.into_parts();
        assert_eq![data, "a"];
        assert_eq![children.len(), 0];
    }

    #[test]
    fn tree_into_parts() {
        let t = owned_tree!["a", ["b"], ["c", ["d"]]];
        let (data, children) = t.into_parts();
        assert_eq![data, "a"];
        assert_eq![children.len(), 2];
        assert_eq![children[0], owned_tree!["b"]];
        assert_eq![children[1], owned_tree!["c", ["d"]]];
    }

    #[test]
    fn debug_fmt() {
        assert_eq!["(\"a\")", format!["{:?}", owned_tree!["a"]]];
        assert_eq!["(\"a\" (\"b\") (\"c\"))", format!["{:?}", owned_tree!["a", ["b"], ["c"]]]];
        assert_eq!["(\"a\" (\"b\") (\"c\" (\"d\") (\"e\")))",
                   format!["{:?}", owned_tree!["a", ["b"], ["c", ["d"], ["e"]]]]];
    }
}
