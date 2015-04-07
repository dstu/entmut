#![feature(alloc)]
#![feature(convert)]
#![feature(core)]

// Basic use cases:
//  - Fixed tree (built once). Handled by Zipper, Tree, Navigator.
//  - Fixed-topology tree (data mutates). Handled by Zipper, Tree, Navigator.
//  - Shared-data tree (topology fixed). Handled by Zipper, Tree, Navigator
//    with RefCell<T> or Mutex<T> for data.
//  - Shared-topology tree (data fixed).
//  - Shared-data, shared-topology tree.

pub mod fixed;
pub mod indirect;

use std::borrow::{Borrow, BorrowMut};
use std::fmt::{Debug, Error, Formatter};
use std::mem;
use std::num::{Int, SignedInt};
use std::ptr;

pub struct Tree<T> {
    data: T,
    children: Vec<Tree<T>>,
}

// Like a stack frame when recursively descending a tree.
struct NavigatorCell<'a, T: 'a> {
    tree: &'a Tree<T>,
    index: usize,
}

pub struct Navigator<'a, T: 'a> {
    here: &'a Tree<T>,
    path: Vec<NavigatorCell<'a, T>>,
}

struct ZipperCell<T> {
    tree: *mut Tree<T>,
    index: usize,
}

pub struct Zipper<'a, T: 'a> {
    here: &'a mut Tree<T>,
    path: Vec<ZipperCell<T>>,
}

enum SiblingOffset {
    Root,
    Underflow,
    Overflow,
    OutOfRange(usize, usize),
    Valid(usize),
}

impl<T> Tree<T> {
    pub fn leaf(data: T) -> Tree<T> {
        Tree {
            data: data,
            children: Vec::new(),
        }
    }

    pub fn data<'s>(&'s self) -> &'s T {
        &self.data
    }

    pub fn data_mut<'s>(&'s mut self) -> &'s mut T {
        &mut self.data
    }

    pub fn children<'s>(&'s self) -> &'s [Tree<T>] {
        self.children.as_ref()
    }

    pub fn children_mut<'s>(&'s mut self) -> &'s mut [Tree<T>] {
        self.children.as_mut_slice()
    }

    pub fn remove_child(&mut self, index: usize) {
        self.children.remove(index);
    }

    pub fn insert_child(&mut self, index: usize, child: Tree<T>) {
        self.children.insert(index, child);
    }

    pub fn pop_child(&mut self) -> Option<Tree<T>> {
        self.children.pop()
    }

    pub fn push_child(&mut self, child: Tree<T>) {
        self.children.push(child);
    }

    pub fn navigator<'s>(&'s self) -> Navigator<'s, T> {
        Navigator { here: self, path: Vec::new(), }
    }

    pub fn zipper<'s>(&'s mut self) -> Zipper<'s, T> {
        Zipper { here: self, path: Vec::new(), }
    }
}

#[macro_export]
macro_rules! tree {
    ($data:expr) => ($crate::Tree::leaf($data));
    ($data:expr, [$($first:tt)*] $(,[$($rest:tt)*])*) =>
        ($crate::Tree { data: $data,
                        children: vec![tree![$($first)*]
                                       $(,tree![$($rest)*])*] });
    ($data:expr, ($($first:tt)*) $(,($($rest:tt)*))*) =>
        ($crate::Tree { data: $data,
                        children: vec![tree![$($first)*]
                                       $(,tree![$($rest)*])*] });
    ($data:expr, {$($first:tt)*} $(,{$($rest:tt)*})*) =>
        ($crate::Tree { data: $data,
                        children: vec![tree![$($first)*]
                                       $(,tree![$($rest)*])*] });
}

impl<T: Debug> Debug for Tree<T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        enum Walk<T> { Down(T), Up, };
        let mut stack = Vec::new();
        try![write!(f, "({:?}", self.data)];
        stack.push(Walk::Up);
        for c in self.children.iter().rev() {
            stack.push(Walk::Down(c));
        }
        loop {
            match stack.pop() {
                None => return Ok(()),
                Some(Walk::Up) => try![write!(f, ")")],
                Some(Walk::Down(t)) => {
                    try![write!(f, " ({:?}", t.data)];
                    stack.push(Walk::Up);
                    for c in t.children.iter().rev() {
                        stack.push(Walk::Down(c));
                    }
                },
            }
        }
    }
}

impl<T: Clone> Clone for Tree<T> {
    fn clone(&self) -> Tree<T> {
        Tree {
            data: self.data.clone(),
            children: self.children.clone(),
        }
    }

    fn clone_from(&mut self, source: &Tree<T>) {
        self.data.clone_from(&source.data);
        self.children.clone_from(&source.children);
    }
}

impl<T: PartialEq> PartialEq for Tree<T> {
    fn eq(&self, other: &Tree<T>) -> bool {
        let mut stack = Vec::new();
        stack.push((self, other));
        loop {
            match stack.pop() {
                Some((x, y)) => {
                    if x.data != y.data {
                        return false;
                    } else if x.children.len() != y.children.len() {
                        return false;
                    } else {
                        let mut xi = x.children.iter();
                        let mut yi = y.children.iter();
                        loop {
                            match (xi.next(), yi.next()) {
                                (Some(xt), Some(yt)) => stack.push((xt, yt)),
                                (None, None) => break,
                                _ => panic!("Tree corruption"),
                            }
                        }
                    }
                },
                None => return true,
            }
        }
    }
}

impl<'a, T: 'a> Navigator<'a, T> {
    pub fn is_root(&self) -> bool {
        self.path.is_empty()
    }

    pub fn is_leaf(&self) -> bool {
        self.here.children.is_empty()
    }

    pub fn to_parent(&mut self) {
        match self.path.pop() {
            None => panic!["already at root"],
            Some(traversal) => self.here = &traversal.tree,
        }
    }

    pub fn to_root(&mut self) {
        loop {
            match self.path.pop() {
                None => return,
                Some(traversal) => self.here = &traversal.tree,
            }
        }
    }

    pub fn tree<'s>(&'s self) -> &'s Tree<T> {
        &self.here
    }

    fn validate_sibling_offset(&self, offset: isize) -> SiblingOffset {
        if self.is_root() {
            return SiblingOffset::Root;
        }
        let offset_abs = offset.abs();
        let parent = &self.path[self.path.len() - 1];
        let new_index =
            if offset_abs < 0 {
                // offset is Int::min_value().
                match parent.index.checked_sub(1) {
                    None => return SiblingOffset::Underflow,
                    Some(x) => match x.checked_sub((offset_abs + 1isize).abs() as usize) {
                        None => return SiblingOffset::Underflow,
                        Some(x) => x,
                    },
                }
            } else {
                if offset < 0 {
                    match parent.index.checked_sub(offset_abs as usize) {
                        None => return SiblingOffset::Underflow,
                        Some(x) => x,
                    }
                } else {
                    match parent.index.checked_add(offset_abs as usize) {
                        None => return SiblingOffset::Overflow,
                        Some(x) => x,
                    }
                }
            };
        if new_index >= parent.tree.children.len() {
            SiblingOffset::OutOfRange(new_index, parent.tree.children.len())
        } else {
            SiblingOffset::Valid(new_index)
        }
    }

    pub fn seek_sibling(&mut self, offset: isize) {
        if offset == 0 {
            return;
        }
        match self.validate_sibling_offset(offset) {
            SiblingOffset::Root => panic!["tree root has no siblings"],
            SiblingOffset::Underflow => panic!["underflow computing sibling index"],
            SiblingOffset::Overflow => panic!["overflow computing sibling index"],
            SiblingOffset::OutOfRange(new_index, siblings) =>
                panic!["cannot address sibling {} (only {} siblings)",
                       new_index, siblings],
            SiblingOffset::Valid(new_index) => {
                let parent_index = self.path.len() - 1;
                let parent = &mut self.path[parent_index];
                parent.index = new_index;
                self.here = &parent.tree.children[new_index]
            },
        }
    }

    pub fn seek_child(&mut self, child_index: usize) {
        assert![child_index < self.here.children.len(),
                "child index {} out of range (only {} children)",
                child_index, self.here.children.len()];
        self.path.push(NavigatorCell { tree: self.here,
                                       index: child_index });
        self.here = &self.here.children[child_index];
    }

    pub fn has_left(&self) -> bool {
        if self.is_root() {
            return false;
        }
        self.path[self.path.len() - 1].index > 0
    }

    pub fn has_right(&self) -> bool {
        if self.is_root() {
            return false;
        }
        let parent = &self.path[self.path.len() - 1];
        parent.index + 1 < parent.tree.children.len()
    }

    pub fn to_left(&mut self) {
        match self.path.pop() {
            None => panic!["root node has no siblings"],
            Some(mut parent) => {
                if parent.index == 0 {
                    panic!["already at leftmost sibling"];
                }
                parent.index -= 1;
                self.here = &parent.tree.children[parent.index];
                self.path.push(parent);
            },
        }
    }

    pub fn to_right(&mut self) {
        match self.path.pop() {
            None => panic!["root node has no siblings"],
            Some(mut parent) => {
                parent.index += 1;
                if parent.index >= parent.tree.children.len() {
                    panic!["already at rightmost sibling"];
                }
                self.here = &parent.tree.children[parent.index];
                self.path.push(parent);
            },
        }
    }
}

impl<'a, T: 'a> Borrow<Tree<T>> for Navigator<'a, T> {
    fn borrow(&self) -> &Tree<T> {
        self.here
    }
}

impl<T> ZipperCell<T> {
    fn tree<'a>(&self) -> &'a Tree<T> {
        unsafe {
            mem::transmute(self.tree)
        }
    }

    fn tree_mut<'a>(&mut self) -> &'a mut Tree<T> {
        unsafe {
            mem::transmute(self.tree)
        }
    }
}

impl<'a, T: 'a> Zipper<'a, T> {
    pub fn is_root(&self) -> bool {
        self.path.is_empty()
    }

    pub fn tree<'s>(&'s self) -> &'s Tree<T> {
        self.here
    }

    pub fn tree_mut<'s>(&'s mut self) -> &'s mut Tree<T> {
        self.here
    }

    pub fn to_root(&mut self) {
        loop {
            match self.path.pop() {
                None => return,
                Some(traversal) => self.here = unsafe {
                    mem::transmute(traversal.tree)
                },
            }
        }
    }

    pub fn to_parent(&mut self) {
        match self.path.pop() {
            None => panic!["already at root"],
            Some(traversal) => self.here = unsafe {
                mem::transmute(traversal.tree)
            },
        }
    }

    pub fn seek_sibling(&mut self, offset: isize) {
        assert![!self.is_root()];
        if offset == 0 {
            return;
        }
        let mut cell = self.path.pop().expect("tree corruption");
        let offset_abs = offset.abs();
        let new_index =
            if offset_abs < 0 {
                // offset is Int::min_value().
                cell.index
                    .checked_sub(1).expect("index underflow")
                    .checked_sub((offset_abs + 1isize).abs() as usize).expect("index underflow")
            } else {
                if offset < 0 {
                    cell.index.checked_sub(offset_abs as usize).expect("index underflow")
                } else {
                    cell.index.checked_add(offset_abs as usize).expect("index overflow")
                }
            };
        {
            let parent: &mut Tree<T> = unsafe {
                mem::transmute(cell.tree)
            };
            assert![new_index < parent.children.len(),
                    "sibling index {} out of range (only {} siblings)",
                    new_index, parent.children.len()];
            self.here = unsafe {
                mem::transmute(&mut parent.children[new_index])
            };
        }
        cell.index = new_index;
        self.path.push(cell);
    }

    pub fn seek_child(&mut self, child_index: usize) {
        assert![child_index < self.here.children.len(),
                "child index {} out of range (only {} children)",
                child_index, self.here.children.len()];
        let raw_here = self.here as *mut Tree<T>;
        self.path.push(ZipperCell { tree: raw_here, index: child_index });
        self.here = unsafe {
            mem::transmute(&mut self.here.children[child_index])
        };
    }

    fn validate_sibling_offset(&self, offset: isize) -> SiblingOffset {
        if self.is_root() {
            return SiblingOffset::Root;
        }
        let offset_abs = offset.abs();
        let parent = &self.path[self.path.len() - 1];
        let new_index =
            if offset_abs < 0 {
                // offset is Int::min_value().
                match parent.index.checked_sub(1) {
                    None => return SiblingOffset::Underflow,
                    Some(x) => match x.checked_sub((offset_abs + 1isize).abs() as usize) {
                        None => return SiblingOffset::Underflow,
                        Some(x) => x,
                    },
                }
            } else {
                if offset < 0 {
                    match parent.index.checked_sub(offset_abs as usize) {
                        None => return SiblingOffset::Underflow,
                        Some(x) => x,
                    }
                } else {
                    match parent.index.checked_add(offset_abs as usize) {
                        None => return SiblingOffset::Overflow,
                        Some(x) => x,
                    }
                }
            };
        if new_index >= parent.tree().children.len() {
            SiblingOffset::OutOfRange(new_index, parent.tree().children.len())
        } else {
            SiblingOffset::Valid(new_index)
        }
    }

    pub fn insert_sibling(&mut self, offset: isize, t: Tree<T>) {
        match self.validate_sibling_offset(offset) {
            SiblingOffset::Root => panic!["tree root has no siblings"],
            SiblingOffset::Underflow => panic!["underflow computing sibling index"],
            SiblingOffset::Overflow => panic!["overflow computing sibling index"],
            SiblingOffset::OutOfRange(new_index, siblings) =>
                panic!["sibling index {} out of range (only {} siblings)",
                       new_index, siblings],
            SiblingOffset::Valid(new_index) => {
                let parent_index = self.path.len() - 1;
                let parent = &mut self.path[parent_index];
                if new_index <= parent.index {
                    parent.index += 1;
                }
                parent.tree_mut().children.insert(new_index, t);
            },
        }
    }

    pub fn insert_child(&mut self, index: usize, t: Tree<T>) {
        assert![index <= self.here.children.len(),
                "child index {} is out of bounds (only {} children)",
                index, self.here.children.len()];
        self.here.children.insert(index, t);
    }

    pub fn push_child(&mut self, t: Tree<T>) {
        self.here.children.push(t);
    }

    pub fn delete(&mut self) {
        match self.path.pop() {
            None => panic!["can't delete root node"],
            Some(mut parent) => {
                let t = parent.tree_mut();
                t.children.remove(parent.index);
                if t.children.is_empty() {
                    self.here = t;
                } else {
                    while parent.index >= t.children.len() {
                        parent.index -= 1;
                    }
                    self.here = &mut t.children[parent.index];
                }
            },
        }
    }

    pub fn delete_child(&mut self, index: usize) {
        assert![index < self.here.children.len(),
                "child index {} is out of bounds (only {} children)",
                index, self.here.children.len()];
        self.here.children.remove(index);
    }

    pub fn delete_sibling(&mut self, offset: isize) {
        if offset == 0isize {
            self.delete();
        } else {
            match self.validate_sibling_offset(offset) {
                SiblingOffset::Root => panic!["tree root has no siblings"],
                SiblingOffset::Underflow => panic!["underflow computing sibling index"],
                SiblingOffset::Overflow => panic!["overflow computing sibling index"],
                SiblingOffset::OutOfRange(new_index, siblings) =>
                    panic!["sibling index {} out of range (only {} siblings)",
                           new_index, siblings],
                SiblingOffset::Valid(new_index) => {
                    let parent_index = self.path.len() - 1;
                    let parent = &mut self.path[parent_index];
                    parent.tree_mut().children.remove(new_index);
                },
            }
        }
    }

    pub fn pop_child(&mut self) -> Option<Tree<T>> {
        self.here.children.pop()
    }

    pub fn swap_sibling(&mut self, offset: isize) {
        if offset == 0isize {
            return;
        }
        match self.validate_sibling_offset(offset) {
            SiblingOffset::Root => panic!["tree root has no siblings"],
            SiblingOffset::Underflow => panic!["underflow computing sibling index"],
            SiblingOffset::Overflow => panic!["overflow computing sibling index"],
            SiblingOffset::OutOfRange(new_index, siblings) =>
                panic!["sibling index {} out of range (only {} siblings)",
                       new_index, siblings],
            SiblingOffset::Valid(new_index) => {
                let parent_index = self.path.len() - 1;
                let parent = &mut self.path[parent_index];
                let t = parent.tree_mut();
                unsafe {
                    ptr::swap(&mut t.children[new_index] as *mut Tree<T>,
                              &mut t.children[parent.index] as *mut Tree<T>);
                };
            },
        }
    }
}

impl<'a, T: 'a> Borrow<Tree<T>> for Zipper<'a, T> {
    fn borrow(&self) -> &Tree<T> {
        self.here
    }
}

impl<'a, T: 'a> BorrowMut<Tree<T>> for Zipper<'a, T> {
    fn borrow_mut(&mut self) -> &mut Tree<T> {
        self.here
    }
}

#[cfg(test)]
mod test {
    use ::Tree;
    
    #[test]
    fn test_leaf() {
        assert![Tree::leaf("hi") == tree!["hi"]];
        assert![Tree { data: "hi", children: Vec::new(), } == tree!["hi"]];
        let leaf = tree!["hi"];
        assert![leaf.data() == &"hi"];
        assert![leaf.children().len() == 0];
    }

    #[test]
    fn test_nested_01() {
        let reference = Tree { data: "hi",
                               children: vec![Tree::leaf("a"),
                                              Tree::leaf("b")] };
        assert![reference == tree!["hi", ["a"], ["b"]]];
    }


    #[test]
    fn test_nested_02() {
        let mut reference = Tree::leaf("hi");
        reference.push_child(Tree::leaf("a"));
        reference.push_child(Tree::leaf("b"));
        assert![reference == tree!["hi", ["a"], ["b"]]];
    }

    #[test]
    fn test_nested_03() {
        let reference = Tree { data: "hi",
                               children: vec![Tree { data: "a",
                                                     children: vec![Tree::leaf("b"),
                                                                    Tree::leaf("c")] },
                                              Tree { data: "d",
                                                     children: vec![Tree {
                                                         data: "e",
                                                         children: vec![Tree::leaf("f"), Tree::leaf("g")]
                                                     }],
                                              }],
        };
        assert![reference == tree!["hi", ["a", ["b"], ["c"]], ["d", ["e", ["f"], ["g"]]]]];
    }

    #[test]
    fn test_debug_format() {
        assert![format!("{:?}", tree!["a"]) == "(\"a\")"];
        assert![format!("{:?}", tree!["hi", ["a", ["b"], ["c"]], ["d", ["e", ["f"], ["g"]]]]) == "(\"hi\" (\"a\" (\"b\") (\"c\")) (\"d\" (\"e\" (\"f\") (\"g\"))))"];
        assert![format!("{:?}", tree!["a", ["b"], ["c"], ["d"], ["e"]]) == "(\"a\" (\"b\") (\"c\") (\"d\") (\"e\"))"];
    }

    #[test]
    fn test_recursive_mutating_bfs() {
        fn mutable_bfs(t: &mut Tree<&str>) {
            if t.children.len() == 0 {
                t.data = "leaf";
            } else {
                for child in &mut t.children {
                    mutable_bfs(child);
                }
            }
        }

        let mut t = tree!["a", ["b", ["c"], ["d"], ["e"]], ["f"]];
        mutable_bfs(&mut t);
        assert_eq![format!["{:?}", t],
                   "(\"a\" (\"b\" (\"leaf\") (\"leaf\") (\"leaf\")) (\"leaf\"))"];
    }
}
