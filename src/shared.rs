use ::{Editor, Nav};
use ::util::{ChildIndex, SiblingIndex};

use std::borrow::Borrow;
use std::cell::{Ref, RefCell, RefMut};
use std::clone::Clone;
use std::fmt;
use std::mem;
use std::ops::Deref;
use std::rc::Rc;
use std::result::Result;

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

    pub fn into_parts(self) -> (T, Vec<Tree<T>>) {
        match Rc::try_unwrap(self.internal) {
            Result::Ok(internal) => (internal.data, internal.children.into_inner()),
            _ => panic!["reference to shared tree element is not unique"],
        }
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

impl<T: PartialEq> PartialEq<Tree<T>> for Tree<T> {
    fn eq(&self, other: &Tree<T>) -> bool {
        let mut x_stack = vec![self.clone()];
        let mut y_stack = vec![other.clone()];
        loop {
            match (x_stack.pop(), y_stack.pop()) {
                (None, None) => return true,
                (Some(x), Some(y)) => {
                    if x.internal.data == y.internal.data {
                        for child in x.internal.children.borrow().iter() {
                            x_stack.push(child.clone());
                        }
                        for child in y.internal.children.borrow().iter() {
                            y_stack.push(child.clone());
                        }
                    } else {
                        return false
                    }
                },
                _ => return false,
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Tree<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        enum PathElement<T> {
            Down(Tree<T>),
            Up,
        }
        try![f.write_str("(")];
        try![self.internal.data.fmt(f)];
        let mut stack = vec![];
        for child in self.internal.children.borrow().iter().rev() {
            stack.push(PathElement::Up);
            stack.push(PathElement::Down(child.clone()));
        }
        loop {
            match stack.pop() {
                Some(PathElement::Down(t)) => {
                    try![f.write_str(" (")];
                    try![t.internal.data.fmt(f)];
                    for child in t.internal.children.borrow().iter().rev() {
                        stack.push(PathElement::Up);
                        stack.push(PathElement::Down(child.clone()));
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

impl<'a, T: 'a> Deref for TreeView<'a, T> {
    type Target = T;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.here().internal.data
    }
}

impl<'a, T: 'a> Nav for TreeView<'a, T> {
    fn seek_sibling(&mut self, offset: isize) -> bool {
        let new_index_result = 
            match self.path.last() {
                None => return offset == 0,
                Some(&(ref siblings, ref index)) =>
                    SiblingIndex::compute(siblings.len(), *index, offset),
            };
        match new_index_result {
            Some(new_index) => {
                let (siblings, _) = self.path.pop().unwrap();
                self.path.push((siblings, new_index));
                return true
            },
            None => return false,
        }
    }

    fn seek_child(&mut self, index: usize) -> bool {
        let child_count = self.child_count();
        match ChildIndex::compute(child_count, index) {
            Some(new_index) => {
                let children = unsafe {
                    mem::transmute(self.here().internal.children.borrow())
                };
                self.path.push((children, new_index));
                return true
            },
            None => return false,
        }
    }

    fn child_count(&self) -> usize {
        self.here().internal.children.borrow().len()
    }

    fn at_root(&self) -> bool {
        self.path.is_empty()
    }

    fn to_parent(&mut self) -> bool {
        match self.path.pop() {
            Some(_) => return true,
            None => return false,
        }
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
    fn seek_sibling(&mut self, offset: isize) -> bool {
        let new_index_result =
            match self.path.last() {
                None => return false,
                Some(&(ref siblings, ref index)) =>
                    SiblingIndex::compute(siblings.len(), *index, offset),
            };
        match new_index_result {
            Some(new_index) => {
                let (siblings, _) = self.path.pop().unwrap();
                self.path.push((siblings, new_index));
                return true
            },
            None => return false,
        }
    }

    fn seek_child(&mut self, index: usize) -> bool {
        let child_count = self.child_count();
        match ChildIndex::compute(child_count, index) {
            Some(new_index) => {
                let children = unsafe {
                    mem::transmute(self.here().internal.children.borrow())
                };
                self.path.push((children, new_index));
                return true
            },
            None => return false,
        }
    }

    fn child_count(&self) -> usize {
        self.here().internal.children.borrow().len()
    }

    fn at_root(&self) -> bool {
        self.path.is_empty()
    }

    fn to_parent(&mut self) -> bool {
        self.path.pop().is_some()
    }

    fn to_root(&mut self) {
        self.path.clear();
    }
}

impl<'a, T: 'a> Borrow<T> for TreeEditor<'a, T> {
    fn borrow(&self) -> &T {
        &self.here().internal.data
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

    fn insert_leaf(&mut self, index: usize, data: T) -> bool {
        self.insert_child(index, Tree::leaf(data))
    }

    fn insert_child(&mut self, index: usize, child: Tree<T>) -> bool {
        match self.path.pop() {
            None => {
                let mut children: RefMut<'a, Vec<Tree<T>>> = unsafe {
                    mem::transmute(self.root.internal.children.borrow_mut())
                };
                match ChildIndex::compute(children.len(), index) {
                    Some(new_index) => {
                        children.insert(new_index, child);
                        self.path.push((children, index));
                        return true
                    },
                    None => return false,
                }
            },
            Some((parent_children, here_index)) => {
                let mut children: RefMut<'a, Vec<Tree<T>>> = unsafe {
                    mem::transmute(parent_children[here_index].internal.children.borrow_mut())
                };
                match ChildIndex::compute(children.len(), index) {
                    Some(new_index) => {
                        children.insert(new_index, child);
                        self.path.push((children, new_index));
                        return true
                    },
                    None => return false,
                }
            },
        }
    }

    fn insert_sibling_leaf(&mut self, offset: isize, data: T) -> bool {
        self.insert_sibling(offset, Tree::leaf(data))
    }

    fn insert_sibling(&mut self, offset: isize, sibling: Tree<T>) -> bool {
        let new_index_result =
            match self.path.last() {
                None => return false,
                Some(&(ref siblings, ref index)) =>
                    SiblingIndex::compute(siblings.len(), *index, offset),
            };
        let (mut siblings, _) = self.path.pop().unwrap();
        match new_index_result {
            Some(new_index) => {
                siblings.insert(new_index, sibling);
                self.path.push((siblings, new_index));
                return true
            },
            None => return false,
        }
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

    fn remove_child(&mut self, index: usize) -> Option<Tree<T>> {
        // TODO return None instead of panicking
        match self.path.pop() {
            None => {
                // At root.
                Some(self.root.internal.children.borrow_mut().remove(index))
            },
            Some((parent_children, here_index)) => {
                let mut children =
                    parent_children[here_index].internal.children.borrow_mut();
                Some(children.remove(here_index))
            },
        }
    }

    fn remove_sibling(&mut self, offset: isize) -> Option<Tree<T>> {
        let index_result = {
            match self.path.last() {
                None => None,
                Some(&(ref parent_children, here_index)) => 
                    SiblingIndex::compute(
                        parent_children.len(), here_index, offset),
            }
        };
        let (mut parent_children, here_index) = self.path.pop().unwrap();
        index_result.map(|index| {
            let removed = parent_children.remove(index);
            let new_index =
                if index > here_index {
                    here_index
                } else {
                    here_index - 1
                };
            self.path.push((parent_children, new_index));
            removed
        })
    }

    fn swap(&mut self, other: &mut Tree<T>) {
        match self.path.last_mut() {
            None => mem::swap(self.root, other),
            Some(&mut (ref mut parent_children, here_index)) =>
                mem::swap(&mut parent_children[here_index], other),
        }
    }

    fn swap_children(&mut self, index_a: usize, index_b: usize) -> bool {
        if index_a >= self.here().internal.children.borrow().len() {
            return false
        }
        if index_b >= self.here().internal.children.borrow().len() {
            return false
        }
        self.here_mut().internal.children.borrow_mut().swap(index_a, index_b);
        return true
    }

    fn swap_siblings(&mut self, offset_a: isize, offset_b: isize) -> bool {
        if self.at_root() {
            return false
        }
        let (mut parent_children, mut here_index) = self.path.pop().unwrap();
        match (SiblingIndex::compute(parent_children.len(), here_index, offset_a),
               SiblingIndex::compute(parent_children.len(), here_index, offset_b)) {
            (Some(index_a), Some(index_b)) => {
                parent_children.swap(index_a, index_b);
                if here_index == index_a {
                    here_index = index_b;
                } else if here_index == index_b {
                    here_index = index_a;
                }
                self.path.push((parent_children, here_index));
                return true
            },
            _ => return false,
        }
    }
}

#[macro_export]
macro_rules! shared_tree {
    ($data:expr) => ($crate::shared::Tree::leaf($data));
    ($data:expr, [$($first:tt)*] $(,[$($rest:tt)*])*) =>
        ($crate::shared::Tree::new($data, vec![shared_tree![$($first)*]
                                               $(,shared_tree![$($rest)*])*]));
}

#[cfg(test)]
mod test {
    use ::shared::Tree;

    #[test]
    fn eq_check() {
        assert_eq![Tree::leaf("a"), Tree::leaf("a")];
        assert![Tree::leaf("a") != Tree::leaf("b")];
        assert_eq![Tree::new("a", vec![Tree::leaf("b"), Tree::leaf("c")]),
                   Tree::new("a", vec![Tree::leaf("b"), Tree::leaf("c")])];
    }

    #[test]
    fn macro_check() {
        assert_eq![Tree::leaf("a"), shared_tree!["a"]];
        assert![Tree::leaf("a") != shared_tree!["b"]];
        assert_eq![Tree::new("a", vec![Tree::leaf("b"), Tree::leaf("c")]),
                   shared_tree!["a", ["b"], ["c"]]];
    }

    #[test]
    fn leaf() {
        let t = Tree::leaf("a");
        assert_eq![t.internal.data, "a"];
        assert_eq![t.internal.children.borrow().len(), 0];
    }

    #[test]
    fn push_child() {
        {
            let mut t = shared_tree!["a"];
            t.push_child(shared_tree!["b"]);
            assert_eq![t, shared_tree!["a", ["b"]]];
        }
        {
            let mut t = shared_tree!["a", ["b"]];
            t.push_child(shared_tree!["c"]);
            assert_eq![t, shared_tree!["a", ["b"], ["c"]]];
        }
        {
            let t = shared_tree!["a", ["b"]];
            t.internal.children.borrow_mut()[0].push_child(shared_tree!["c"]);
            assert_eq![t, shared_tree!["a", ["b", ["c"]]]];
        }
    }

    #[test]
    #[should_panic]
    fn remove_child_panics_no_children() {
        shared_tree!["a"].remove_child(0);
    }

    #[test]
    #[should_panic]
    fn remove_child_panics_bad_index() {
        shared_tree!["a", ["b"], ["c"]].remove_child(2);
    }

    #[test]
    fn remove_child() {
        {
            let mut t = shared_tree!["a", ["b"]];
            t.remove_child(0);
            assert_eq![t, shared_tree!["a"]];
        }
        {
            let mut t = shared_tree!["a", ["b"], ["c"]];
            t.remove_child(0);
            assert_eq![t, shared_tree!["a", ["c"]]];
            t.remove_child(0);
            assert_eq![t, shared_tree!["a"]];
        }
        {
            let mut t = shared_tree!["a", ["b"], ["c"]];
            t.remove_child(1);
            assert_eq![t, shared_tree!["a", ["b"]]];
            t.remove_child(0);
            assert_eq![t, shared_tree!["a"]];
        }
    }

    #[test]
    #[should_panic]
    fn insert_child_panics_no_children() {
        shared_tree!["a"].insert_child(1, shared_tree!["b"]);
    }

    #[test]
    #[should_panic]
    fn insert_child_panics_bad_index() {
        shared_tree!["a", ["b"]].insert_child(2, shared_tree!["c"]);
    }

    #[test]
    fn insert_child_at_leaf() {
        let mut t = shared_tree!["a"];
        t.insert_child(0, shared_tree!["b"]);
        assert_eq![t, shared_tree!["a", ["b"]]];
    }

    #[test]
    fn insert_child_at_start() {
        let mut t = shared_tree!["a", ["b"], ["c", ["d"]], ["e"]];
        t.insert_child(0, shared_tree!["aa"]);
        assert_eq![t, shared_tree!["a", ["aa"], ["b"], ["c", ["d"]], ["e"]]];
    }

    #[test]
    fn insert_child_at_end() {
        let mut t = shared_tree!["a", ["b"], ["c", ["d"]], ["e"]];
        t.insert_child(3, shared_tree!["aa"]);
        assert_eq![t, shared_tree!["a", ["b"], ["c", ["d"]], ["e"], ["aa"]]];
    }

    #[test]
    fn insert_child_at_middle() {
        let mut t = shared_tree!["a", ["b"], ["c", ["d"]], ["e"]];
        t.insert_child(2, shared_tree!["aa"]);
        assert_eq![t.clone(), shared_tree!["a", ["b"], ["c", ["d"]], ["aa"], ["e"]]];
    }

    #[test]
    fn leaf_into_parts() {
        let t = shared_tree!["a"];
        let (data, children) = t.into_parts();
        assert_eq![data, "a"];
        assert![children.is_empty()];
    }

    #[test]
    fn tree_into_parts() {
        let t = shared_tree!["a", ["b"], ["c", ["d"]]];
        let (data, children) = t.into_parts();
        assert_eq![data, "a"];
        assert_eq![children.len(), 2];
        assert_eq![children[0].clone(), shared_tree!["b"]];
        assert_eq![children[1].clone(), shared_tree!["c", ["d"]]];
    }

    #[test]
    #[should_panic]
    #[allow(unused_variables)]
    fn into_parts_panics_when_shared() {
        let t = shared_tree!["a"];
        let u = t.clone();
        let _ = t.into_parts();
    }

    #[test]
    fn debug_fmt() {
        assert_eq!["(\"a\")", format!["{:?}", shared_tree!["a"]]];
        assert_eq!["(\"a\" (\"b\") (\"c\"))", format!["{:?}", shared_tree!["a", ["b"], ["c"]]]];
        assert_eq!["(\"a\" (\"b\") (\"c\" (\"d\") (\"e\")))",
                   format!["{:?}", shared_tree!["a", ["b"], ["c", ["d"], ["e"]]]]];
    }
}
