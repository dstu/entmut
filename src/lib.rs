//! Tree structure implementations and common traits for manipulating them.

// Basic use cases:
//  - Fixed tree (built once). Handled by Zipper, Tree, Nav.
//  - Fixed-topology tree (data mutates). Handled by Zipper, Tree, Nav.
//  - Shared-data tree (topology fixed). Handled by Zipper, Tree, Nav with
//    RefCell<T> or Mutex<T> for data.
//  - Shared-topology tree (data fixed).
//  - Shared-data, shared-topology tree.

// For std::intrinsics::unreachable.
#![feature(core)]

/// Fixed-layout trees with good memory locality guarantees.
pub mod fixed;
/// Single-ownership trees wherein a parent owns its children.
pub mod owned;
/// Heap-allocated, reference-counted trees that can be shared freely.
pub mod shared;
/// Tree traversal methods and interfaces.
pub mod traversal;
/// Internal utilities.
mod util;

/// Navigable focus-based view of a tree.
///
/// This trait defines a view of a tree that is analogous to a sequential
/// iterator providing read-only pointers into a structure. It can be thought of
/// as having a particular tree node as its focus, relative to which operations
/// are performed.Methods are provided for walking the tree and updating which
/// node is the focus.
///
/// If you have worked with
/// [zippers](http://en.wikipedia.org/wiki/Zipper_(data_structure)), this should
/// seem familiar.
///
/// For access to data at tree nodes, implementing types should also implement
/// `std::borrow::Borrow` or `std::borrow::BorrowMut`.
///
/// The read-only nature of this view does not guarantee immutability or thread
/// safety. Implementing types may permit mutation of tree data (whether by
/// implementing `std::borrow::BorrowMut`, implementing `std::borrow::Borrow`
/// and having `RefCell` data, or otherwise), which may in turn cause arbitrary
/// modififixcations in the underlying representation of the tree structure, such
/// as reallocations. The `Editor` trait, which extends this one, permits
/// modification of the tree topology.
///
/// To make it convenient to navigate through a tree and retain pointers along
/// the way, it is recommended that implementors also provide an implementation
/// of `std::clone::Clone`. For mutable types that also implement
/// `std::borrow::BorrowMut`, which may require a read-write borrow of an
/// underlying structure, this may not be possible.
pub trait Nav {
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
    /// right. (An offset of 0 is a no-op.) Panics if this is the tree root or
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
}

/// Navigable view of a tree, with support for modifying the tree's topology.
///
/// This trait defines a view of a tree that is analogous to a sequential
/// iterator which supports insertions and deletions. It extends `Nav` by
/// providing an interface for tree modification operations.
pub trait Editor: Nav {
    /// The type of tree node data, usually the `T` of some `Tree<T>`.
    type Data;

    /// The tree type that is associated with operations that insert or remove
    /// subtrees. This is typically the implementing tree type that the `Editor`
    /// provides a view of.
    type Tree;

    /// Creates a new leaf with the given data at the logical end of the
    /// children of the current focus and focuses on it.
    fn push_leaf(&mut self, data: <Self as Editor>::Data);

    /// Adds `child` to the logical end of the children of the current focus and
    /// focuses on it.
    fn push_child(&mut self, child: <Self as Editor>::Tree);

    /// Inserts a new leaf with the given data at the given position in the
    /// current focus's children and focuses on it.
    fn insert_leaf(&mut self, index: usize, data: <Self as Editor>::Data);

    /// Inserts `child` at the given position in the current focus's children
    /// and focuses on it.
    fn insert_child(
        &mut self, index: usize, child: <Self as Editor>::Tree);

    /// Inserts a new leaf with the given data at the position an offset by the
    /// given amount from the current focus and focuses on it. Panics if the
    /// offset is invalid.
    fn insert_sibling_leaf(
        &mut self, offset: isize, data: <Self as Editor>::Data);

    /// Inserts `sibling` at the given offset relative to the current focus and
    /// focuses on it. Panics if the offset is invalid.
    fn insert_sibling(
        &mut self, offset: isize, sibling: <Self as Editor>::Tree);

    /// Removes the focus node and returns the subtree rooted at it. Focus
    /// changes to (in order of preference) the focus's left sibling, its right
    /// sibling (if there is no left sibling), or its parent (if there are no
    /// siblings).
    fn remove(&mut self) -> <Self as Editor>::Tree;

    /// Removes the child at the given index and returns the subtree rooted at
    /// it.
    fn remove_child(&mut self, index: usize) -> <Self as Editor>::Tree;

    /// Removes the sibling at the given offset and returns the subtree rooted
    /// at it.
    fn remove_sibling(&mut self, offset: isize) -> <Self as Editor>::Tree;

    /// Swaps the focus node and `other`.
    fn swap(&mut self, other: &mut <Self as Editor>::Tree);

    /// Swaps the children at the given indices. If the indices are equal, this
    /// is a no-op. If either index corresponds to the focus, focus follows it
    /// after the swap.
    fn swap_children(&mut self, index_a: usize, index_b: usize);

    /// Swaps the sibling nodes at the given offsets. If the offsets are equal,
    /// this is a no-op. If either offset is 0 (corresponding to the focus),
    /// focus follows it after the swap.
    fn swap_siblings(&mut self, offset_a: isize, offset_b: isize);
}

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
