//! Tree structure implementations and common traits for manipulating them.

// Basic use cases:
//  - Fixed tree (built once). Handled by Zipper, Tree, Nav.
//  - Fixed-topology tree (data mutates). Handled by Zipper, Tree, Nav.
//  - Shared-data tree (topology fixed). Handled by Zipper, Tree, Nav with
//    RefCell<T> or Mutex<T> for data.
//  - Shared-topology tree (data fixed).
//  - Shared-data, shared-topology tree.

pub mod fixed;
// pub mod linked;
pub mod owned;
pub mod shared;

mod util;

use std::mem;
use std::ops::Deref;

/// Accessor trait that provides a fixed-lifetime read-only reference to
/// data.
///
/// This is used by read-only views of a tree structure, with the lifetime of
/// the guard pinned to the lifetime of the view. If a tree implementation
/// allows internal mutability (via the use of `RefCell` or otherwise), then
/// this does not guarantee that the data will not change.
pub trait Guard<'a, T: 'a> {
    fn super_deref<'s>(&'s self) -> &'a T;
}

impl<'a, T: 'a> Deref for Guard<'a, T> {
    type Target = T;
    fn deref<'s>(&'s self) -> &'s T {
        unsafe {
            mem::transmute(self.super_deref())
        }
    }
}

/// Navigable fixed-topology view of a tree.
///
/// This trait defines a view of a tree that is analogous to a sequential
/// iterator providing read-only pointers into a structure. At any given point
/// in time, it can be thought of as pointing to a particular tree node. Methods
/// are provided for walking the tree and updating which node is pointed at. A
/// guarded reference to the data at a node can be obtained at any time, with
/// the lifetime of the reference good for the lifetime of the view of the tree.
///
/// If you have worked with
/// [zippers](http://en.wikipedia.org/wiki/Zipper_(data_structure)), this should
/// seem familiar.
///
/// The read-only nature of this view does not guarantee immutability or thread
/// safety. An internally mutable type for `Data` (like `std::cell::RefCell<T>`)
/// will permit updates to tree data through this view. The tree topology,
/// however, should stay fixed.
///
/// Implementations of this trait should have their lifetime parameter
/// constrained by a read-only borrow of a tree structure. It should be safe to
/// create multiple views of the same structure.
///
/// To make it convenient to navigate through a tree and retain pointers along
/// the way, it is recommended that implementors also provide an implementation
/// of `std::clone::Clone`.
pub trait Nav<'a> {
    /// Type of data structures held at tree nodes.
    ///
    /// E.g., the `T` of some `Tree<T>`.
    type Data;

    /// Concrete guard implementation.
    type DataGuard: Guard<'a, <Self as Nav<'a>>::Data>;

    /// Returns the number of children of the current node.
    fn child_count(&self) -> usize;

    /// Returns `true` iff the current node is a leaf (i.e., it has no
    /// children).
    fn at_leaf(&self) -> bool {
        self.child_count() == 0
    }

    /// Returns `true` iff the current node is the tree root (i.e., it has no
    /// parent).
    fn at_root(&self) -> bool;

    /// Navigates to the sibling at `offset`, for which negative values indicate
    /// navigating to the left of this node's location and positive value to the
    /// right. An offset of 0 is a no-op. Panics if this is the tree root or
    /// `offset` resolves to a nonexistant sibling.
    fn seek_sibling(&mut self, offset: isize);

    /// Navigates to the child at at the given index. Panics if there are no
    /// children to navigate to or `index` resolves to a nonexistant child.
    fn seek_child(&mut self, index: usize);

    /// Navigates to this node's parent. Panics if this is the root.
    fn to_parent(&mut self);

    /// Navigates to the tree's root. If this navigator is already pointing at
    /// the tree root, this is a no-op.
    ///
    /// The default implementation of this method repeatedly calls
    /// `to_parent`. Implementors may wish to provide a more efficient method.
    fn to_root(&mut self) {
        while ! self.at_root() {
            self.to_parent();
        }
    }

    /// Returns this node's data. The guard that is returned should be viable
    /// for the lifetime of this view.
    fn data(&self) -> <Self as Nav<'a>>::DataGuard;
}

// pub trait Treeish {
//     type Data;
//     type DataGuard: Deref<Target=<Self as Treeish>::Data>;
//     type Childish: Deref<Target=Self>;
//     fn data(&self) -> <Self as Treeish>::DataGuard;
//     fn child(&self, index: usize) -> <Self as Treeish>::Childish;
//     fn child_count(&self) -> usize;
//     // fn children(&self) -> <Self as Treeish>::Children;
//     // fn children_mut(&mut self) -> <Self as Treeish>::ChildrenMut;
// }

// Like a stack frame when recursively descending a tree.
// struct NavigatorCell<'a, T: 'a> {
//     tree: &'a Tree<T>,
//     index: usize,
// }

// pub struct Navigator<'a, T: 'a> {
//     here: &'a Tree<T>,
//     path: Vec<NavigatorCell<'a, T>>,
// }

// struct ZipperCell<T> {
//     tree: *mut Tree<T>,
//     index: usize,
// }

// pub struct Zipper<'a, T: 'a> {
//     here: &'a mut Tree<T>,
//     path: Vec<ZipperCell<T>>,
// }

// impl<T> Tree<T> {
//     pub fn leaf(data: T) -> Tree<T> {
//         Tree {
//             data: data,
//             children: Vec::new(),
//         }
//     }

//     pub fn data<'s>(&'s self) -> &'s T {
//         &self.data
//     }

//     pub fn data_mut<'s>(&'s mut self) -> &'s mut T {
//         &mut self.data
//     }

//     pub fn children<'s>(&'s self) -> &'s [Tree<T>] {
//         self.children.as_ref()
//     }

//     pub fn children_mut<'s>(&'s mut self) -> &'s mut [Tree<T>] {
//         self.children.as_mut_slice()
//     }

//     pub fn remove_child(&mut self, index: usize) {
//         self.children.remove(index);
//     }

//     pub fn insert_child(&mut self, index: usize, child: Tree<T>) {
//         self.children.insert(index, child);
//     }

//     pub fn pop_child(&mut self) -> Option<Tree<T>> {
//         self.children.pop()
//     }

//     pub fn push_child(&mut self, child: Tree<T>) {
//         self.children.push(child);
//     }

//     pub fn navigator<'s>(&'s self) -> Navigator<'s, T> {
//         Navigator { here: self, path: Vec::new(), }
//     }

//     pub fn zipper<'s>(&'s mut self) -> Zipper<'s, T> {
//         Zipper { here: self, path: Vec::new(), }
//     }
// }

// #[macro_export]
// macro_rules! tree {
//     ($data:expr) => ($crate::Tree::leaf($data));
//     ($data:expr, [$($first:tt)*] $(,[$($rest:tt)*])*) =>
//         ($crate::Tree { data: $data,
//                         children: vec![tree![$($first)*]
//                                        $(,tree![$($rest)*])*] });
//     ($data:expr, ($($first:tt)*) $(,($($rest:tt)*))*) =>
//         ($crate::Tree { data: $data,
//                         children: vec![tree![$($first)*]
//                                        $(,tree![$($rest)*])*] });
//     ($data:expr, {$($first:tt)*} $(,{$($rest:tt)*})*) =>
//         ($crate::Tree { data: $data,
//                         children: vec![tree![$($first)*]
//                                        $(,tree![$($rest)*])*] });
// }

// impl<'a, T: Debug + 'a, G: Guard<'a, T>> Debug for Nav<'a, Data=T, DataGuard=G> {
//     fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
//         enum Walk<T> { Down(T), Up, };
//         let mut stack = Vec::new();
//         try![write!(f, "({:?}", self.data().deref())];
//         stack.push(Walk::Up);
//         for c in self.children.iter().rev() {
//             stack.push(Walk::Down(c));
//         }
//         loop {
//             match stack.pop() {
//                 None => return Ok(()),
//                 Some(Walk::Up) => try![write!(f, ")")],
//                 Some(Walk::Down(t)) => {
//                     try![write!(f, " ({:?}", *t.data().deref())];
//                     stack.push(Walk::Up);
//                     for c in t.children.iter().rev() {
//                         stack.push(Walk::Down(c));
//                     }
//                 },
//             }
//         }
//     }
// }

// impl<T: Clone> Clone for Tree<T> {
//     fn clone(&self) -> Tree<T> {
//         Tree {
//             data: self.data.clone(),
//             children: self.children.clone(),
//         }
//     }

//     fn clone_from(&mut self, source: &Tree<T>) {
//         self.data.clone_from(&source.data);
//         self.children.clone_from(&source.children);
//     }
// }

// impl<T: PartialEq> PartialEq for Tree<T> {
//     fn eq(&self, other: &Tree<T>) -> bool {
//         let mut stack = Vec::new();
//         stack.push((self, other));
//         loop {
//             match stack.pop() {
//                 Some((x, y)) => {
//                     if x.data != y.data {
//                         return false;
//                     } else if x.children.len() != y.children.len() {
//                         return false;
//                     } else {
//                         let mut xi = x.children.iter();
//                         let mut yi = y.children.iter();
//                         loop {
//                             match (xi.next(), yi.next()) {
//                                 (Some(xt), Some(yt)) => stack.push((xt, yt)),
//                                 (None, None) => break,
//                                 _ => panic!("Tree corruption"),
//                             }
//                         }
//                     }
//                 },
//                 None => return true,
//             }
//         }
//     }
// }

// impl<'a, T: 'a> Navigator<'a, T> {
//     pub fn is_root(&self) -> bool {
//         self.path.is_empty()
//     }

//     pub fn is_leaf(&self) -> bool {
//         self.here.children.is_empty()
//     }

//     pub fn to_parent(&mut self) {
//         match self.path.pop() {
//             None => panic!["already at root"],
//             Some(traversal) => self.here = &traversal.tree,
//         }
//     }

//     pub fn to_root(&mut self) {
//         loop {
//             match self.path.pop() {
//                 None => return,
//                 Some(traversal) => self.here = &traversal.tree,
//             }
//         }
//     }

//     pub fn tree<'s>(&'s self) -> &'s Tree<T> {
//         &self.here
//     }


//     pub fn seek_sibling(&mut self, offset: isize) {
//         if offset == 0 {
//             return;
//         }
//         match self.validate_sibling_offset(offset) {
//             SiblingOffset::Root => panic!["tree root has no siblings"],
//             SiblingOffset::Underflow => panic!["underflow computing sibling index"],
//             SiblingOffset::Overflow => panic!["overflow computing sibling index"],
//             SiblingOffset::OutOfRange(new_index, siblings) =>
//                 panic!["cannot address sibling {} (only {} siblings)",
//                        new_index, siblings],
//             SiblingOffset::Valid(new_index) => {
//                 let parent_index = self.path.len() - 1;
//                 let parent = &mut self.path[parent_index];
//                 parent.index = new_index;
//                 self.here = &parent.tree.children[new_index]
//             },
//         }
//     }

//     pub fn seek_child(&mut self, child_index: usize) {
//         assert![child_index < self.here.children.len(),
//                 "child index {} out of range (only {} children)",
//                 child_index, self.here.children.len()];
//         self.path.push(NavigatorCell { tree: self.here,
//                                        index: child_index });
//         self.here = &self.here.children[child_index];
//     }

//     pub fn has_left(&self) -> bool {
//         if self.is_root() {
//             return false;
//         }
//         self.path[self.path.len() - 1].index > 0
//     }

//     pub fn has_right(&self) -> bool {
//         if self.is_root() {
//             return false;
//         }
//         let parent = &self.path[self.path.len() - 1];
//         parent.index + 1 < parent.tree.children.len()
//     }

//     pub fn to_left(&mut self) {
//         match self.path.pop() {
//             None => panic!["root node has no siblings"],
//             Some(mut parent) => {
//                 if parent.index == 0 {
//                     panic!["already at leftmost sibling"];
//                 }
//                 parent.index -= 1;
//                 self.here = &parent.tree.children[parent.index];
//                 self.path.push(parent);
//             },
//         }
//     }

//     pub fn to_right(&mut self) {
//         match self.path.pop() {
//             None => panic!["root node has no siblings"],
//             Some(mut parent) => {
//                 parent.index += 1;
//                 if parent.index >= parent.tree.children.len() {
//                     panic!["already at rightmost sibling"];
//                 }
//                 self.here = &parent.tree.children[parent.index];
//                 self.path.push(parent);
//             },
//         }
//     }
// }

// impl<'a, T: 'a> Borrow<Tree<T>> for Navigator<'a, T> {
//     fn borrow(&self) -> &Tree<T> {
//         self.here
//     }
// }

// impl<T> ZipperCell<T> {
//     fn tree<'a>(&self) -> &'a Tree<T> {
//         unsafe {
//             mem::transmute(self.tree)
//         }
//     }

//     fn tree_mut<'a>(&mut self) -> &'a mut Tree<T> {
//         unsafe {
//             mem::transmute(self.tree)
//         }
//     }
// }

// impl<'a, T: 'a> Zipper<'a, T> {
//     pub fn is_root(&self) -> bool {
//         self.path.is_empty()
//     }

//     pub fn tree<'s>(&'s self) -> &'s Tree<T> {
//         self.here
//     }

//     pub fn tree_mut<'s>(&'s mut self) -> &'s mut Tree<T> {
//         self.here
//     }

//     pub fn to_root(&mut self) {
//         loop {
//             match self.path.pop() {
//                 None => return,
//                 Some(traversal) => self.here = unsafe {
//                     mem::transmute(traversal.tree)
//                 },
//             }
//         }
//     }

//     pub fn to_parent(&mut self) {
//         match self.path.pop() {
//             None => panic!["already at root"],
//             Some(traversal) => self.here = unsafe {
//                 mem::transmute(traversal.tree)
//             },
//         }
//     }

//     pub fn seek_sibling(&mut self, offset: isize) {
//         assert![!self.is_root()];
//         if offset == 0 {
//             return;
//         }
//         let mut cell = self.path.pop().expect("tree corruption");
//         let offset_abs = offset.abs();
//         let new_index =
//             if offset_abs < 0 {
//                 // offset is Int::min_value().
//                 cell.index
//                     .checked_sub(1).expect("index underflow")
//                     .checked_sub((offset_abs + 1isize).abs() as usize).expect("index underflow")
//             } else {
//                 if offset < 0 {
//                     cell.index.checked_sub(offset_abs as usize).expect("index underflow")
//                 } else {
//                     cell.index.checked_add(offset_abs as usize).expect("index overflow")
//                 }
//             };
//         {
//             let parent: &mut Tree<T> = unsafe {
//                 mem::transmute(cell.tree)
//             };
//             assert![new_index < parent.children.len(),
//                     "sibling index {} out of range (only {} siblings)",
//                     new_index, parent.children.len()];
//             self.here = unsafe {
//                 mem::transmute(&mut parent.children[new_index])
//             };
//         }
//         cell.index = new_index;
//         self.path.push(cell);
//     }

//     pub fn seek_child(&mut self, child_index: usize) {
//         assert![child_index < self.here.children.len(),
//                 "child index {} out of range (only {} children)",
//                 child_index, self.here.children.len()];
//         let raw_here = self.here as *mut Tree<T>;
//         self.path.push(ZipperCell { tree: raw_here, index: child_index });
//         self.here = unsafe {
//             mem::transmute(&mut self.here.children[child_index])
//         };
//     }

//     fn validate_sibling_offset(&self, offset: isize) -> SiblingOffset {
//         if self.is_root() {
//             return SiblingOffset::Root;
//         }
//         let offset_abs = offset.abs();
//         let parent = &self.path[self.path.len() - 1];
//         let new_index =
//             if offset_abs < 0 {
//                 // offset is Int::min_value().
//                 match parent.index.checked_sub(1) {
//                     None => return SiblingOffset::Underflow,
//                     Some(x) => match x.checked_sub((offset_abs + 1isize).abs() as usize) {
//                         None => return SiblingOffset::Underflow,
//                         Some(x) => x,
//                     },
//                 }
//             } else {
//                 if offset < 0 {
//                     match parent.index.checked_sub(offset_abs as usize) {
//                         None => return SiblingOffset::Underflow,
//                         Some(x) => x,
//                     }
//                 } else {
//                     match parent.index.checked_add(offset_abs as usize) {
//                         None => return SiblingOffset::Overflow,
//                         Some(x) => x,
//                     }
//                 }
//             };
//         if new_index >= parent.tree().children.len() {
//             SiblingOffset::OutOfRange(new_index, parent.tree().children.len())
//         } else {
//             SiblingOffset::Valid(new_index)
//         }
//     }

//     pub fn insert_sibling(&mut self, offset: isize, t: Tree<T>) {
//         match self.validate_sibling_offset(offset) {
//             SiblingOffset::Root => panic!["tree root has no siblings"],
//             SiblingOffset::Underflow => panic!["underflow computing sibling index"],
//             SiblingOffset::Overflow => panic!["overflow computing sibling index"],
//             SiblingOffset::OutOfRange(new_index, siblings) =>
//                 panic!["sibling index {} out of range (only {} siblings)",
//                        new_index, siblings],
//             SiblingOffset::Valid(new_index) => {
//                 let parent_index = self.path.len() - 1;
//                 let parent = &mut self.path[parent_index];
//                 if new_index <= parent.index {
//                     parent.index += 1;
//                 }
//                 parent.tree_mut().children.insert(new_index, t);
//             },
//         }
//     }

//     pub fn insert_child(&mut self, index: usize, t: Tree<T>) {
//         assert![index <= self.here.children.len(),
//                 "child index {} is out of bounds (only {} children)",
//                 index, self.here.children.len()];
//         self.here.children.insert(index, t);
//     }

//     pub fn push_child(&mut self, t: Tree<T>) {
//         self.here.children.push(t);
//     }

//     pub fn delete(&mut self) {
//         match self.path.pop() {
//             None => panic!["can't delete root node"],
//             Some(mut parent) => {
//                 let t = parent.tree_mut();
//                 t.children.remove(parent.index);
//                 if t.children.is_empty() {
//                     self.here = t;
//                 } else {
//                     while parent.index >= t.children.len() {
//                         parent.index -= 1;
//                     }
//                     self.here = &mut t.children[parent.index];
//                 }
//             },
//         }
//     }

//     pub fn delete_child(&mut self, index: usize) {
//         assert![index < self.here.children.len(),
//                 "child index {} is out of bounds (only {} children)",
//                 index, self.here.children.len()];
//         self.here.children.remove(index);
//     }

//     pub fn delete_sibling(&mut self, offset: isize) {
//         if offset == 0isize {
//             self.delete();
//         } else {
//             match self.validate_sibling_offset(offset) {
//                 SiblingOffset::Root => panic!["tree root has no siblings"],
//                 SiblingOffset::Underflow => panic!["underflow computing sibling index"],
//                 SiblingOffset::Overflow => panic!["overflow computing sibling index"],
//                 SiblingOffset::OutOfRange(new_index, siblings) =>
//                     panic!["sibling index {} out of range (only {} siblings)",
//                            new_index, siblings],
//                 SiblingOffset::Valid(new_index) => {
//                     let parent_index = self.path.len() - 1;
//                     let parent = &mut self.path[parent_index];
//                     parent.tree_mut().children.remove(new_index);
//                 },
//             }
//         }
//     }

//     pub fn pop_child(&mut self) -> Option<Tree<T>> {
//         self.here.children.pop()
//     }

//     pub fn swap_sibling(&mut self, offset: isize) {
//         if offset == 0isize {
//             return;
//         }
//         match self.validate_sibling_offset(offset) {
//             SiblingOffset::Root => panic!["tree root has no siblings"],
//             SiblingOffset::Underflow => panic!["underflow computing sibling index"],
//             SiblingOffset::Overflow => panic!["overflow computing sibling index"],
//             SiblingOffset::OutOfRange(new_index, siblings) =>
//                 panic!["sibling index {} out of range (only {} siblings)",
//                        new_index, siblings],
//             SiblingOffset::Valid(new_index) => {
//                 let parent_index = self.path.len() - 1;
//                 let parent = &mut self.path[parent_index];
//                 let t = parent.tree_mut();
//                 unsafe {
//                     ptr::swap(&mut t.children[new_index] as *mut Tree<T>,
//                               &mut t.children[parent.index] as *mut Tree<T>);
//                 };
//             },
//         }
//     }
// }

// impl<'a, T: 'a> Borrow<Tree<T>> for Zipper<'a, T> {
//     fn borrow(&self) -> &Tree<T> {
//         self.here
//     }
// }

// impl<'a, T: 'a> BorrowMut<Tree<T>> for Zipper<'a, T> {
//     fn borrow_mut(&mut self) -> &mut Tree<T> {
//         self.here
//     }
// }

// #[cfg(test)]
// mod test {
//     use ::Tree;
    
//     #[test]
//     fn test_leaf() {
//         assert![Tree::leaf("hi") == tree!["hi"]];
//         assert![Tree { data: "hi", children: Vec::new(), } == tree!["hi"]];
//         let leaf = tree!["hi"];
//         assert![leaf.data() == &"hi"];
//         assert![leaf.children().len() == 0];
//     }

//     #[test]
//     fn test_nested_01() {
//         let reference = Tree { data: "hi",
//                                children: vec![Tree::leaf("a"),
//                                               Tree::leaf("b")] };
//         assert![reference == tree!["hi", ["a"], ["b"]]];
//     }


//     #[test]
//     fn test_nested_02() {
//         let mut reference = Tree::leaf("hi");
//         reference.push_child(Tree::leaf("a"));
//         reference.push_child(Tree::leaf("b"));
//         assert![reference == tree!["hi", ["a"], ["b"]]];
//     }

//     #[test]
//     fn test_nested_03() {
//         let reference = Tree { data: "hi",
//                                children: vec![Tree { data: "a",
//                                                      children: vec![Tree::leaf("b"),
//                                                                     Tree::leaf("c")] },
//                                               Tree { data: "d",
//                                                      children: vec![Tree {
//                                                          data: "e",
//                                                          children: vec![Tree::leaf("f"), Tree::leaf("g")]
//                                                      }],
//                                               }],
//         };
//         assert![reference == tree!["hi", ["a", ["b"], ["c"]], ["d", ["e", ["f"], ["g"]]]]];
//     }

//     #[test]
//     fn test_debug_format() {
//         assert![format!("{:?}", tree!["a"]) == "(\"a\")"];
//         assert![format!("{:?}", tree!["hi", ["a", ["b"], ["c"]], ["d", ["e", ["f"], ["g"]]]]) == "(\"hi\" (\"a\" (\"b\") (\"c\")) (\"d\" (\"e\" (\"f\") (\"g\"))))"];
//         assert![format!("{:?}", tree!["a", ["b"], ["c"], ["d"], ["e"]]) == "(\"a\" (\"b\") (\"c\") (\"d\") (\"e\"))"];
//     }

//     #[test]
//     fn test_recursive_mutating_bfs() {
//         fn mutable_bfs(t: &mut Tree<&str>) {
//             if t.children.len() == 0 {
//                 t.data = "leaf";
//             } else {
//                 for child in &mut t.children {
//                     mutable_bfs(child);
//                 }
//             }
//         }

//         let mut t = tree!["a", ["b", ["c"], ["d"], ["e"]], ["f"]];
//         mutable_bfs(&mut t);
//         assert_eq![format!["{:?}", t],
//                    "(\"a\" (\"b\" (\"leaf\") (\"leaf\") (\"leaf\")) (\"leaf\"))"];
//     }
// }
