use ::{Guard, Nav, View, Zipper};
use ::util::{ChildIndex, SiblingIndex};

use std::cell::UnsafeCell;
use std::clone::Clone;
use std::iter::Iterator;
use std::mem;

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

    // TODO: loop instead of recurring to avoid blowing the stack.
    pub fn from_traversal<I: Iterator<Item=(T, I)>>(data: T, children: I) -> Self {
        let mut t = Tree { data: data, children: vec![], };
        for (child_data, child_children) in children {
            t.children.push(Tree::from_traversal(child_data, child_children));
        }
        t.children.shrink_to_fit();
        return t;
    }

    pub fn view<'s>(&'s self) -> TreeView<'s, T> {
        TreeView::new(self)
    }
}

pub struct DataGuard<'a, T: 'a> {
    tree: &'a Tree<T>,
}

impl<'a, T: 'a> Guard<'a, T> for DataGuard<'a, T> {
    fn super_deref<'s>(&'s self) -> &'a T {
        &self.tree.data
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

impl<'a, T: 'a> View<'a> for TreeView<'a, T> {
    type Data = T;
    type DataGuard = DataGuard<'a, T>;
    fn data(&self) -> DataGuard<'a, T> {
        DataGuard { tree: self.here, }
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
        let new_index =
            ChildIndex::compute(self.here.children.len(), index).unwrap();
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

enum ZipperCell<T> {
    Root(T, Vec<Tree<T>>),
    Nonroot(Vec<Tree<T>>, usize),
}

pub struct Z<T> {
    path: Vec<(Vec<Tree<T>>, Tree<T>, Vec<Tree<T>>)>,
    lefts: Vec<Tree<T>>,
    here: Tree<T>,
    rights: Vec<Tree<T>>,
}

pub struct TreeZipper<T> {
    root: Tree<T>,
    path: Vec<(*mut Tree<T>, usize)>,
    here: *mut Tree<T>,
}

impl<T> TreeZipper<T> {
    fn new(root: Tree<T>) -> Self {
        TreeZipper { here: root, path: vec![], }
    }
    
    // fn here(&self) -> &Tree<T> {
    //     let &(_, ref children, ref index) = &self.path[self.path.len() - 1];
    //     &children[*index]
    // }

    // fn here_mut(&mut self) -> &mut Tree<T> {
    //     let last_index = self.path.len() - 1;
    //     let &mut (ref mut children, ref index) = &mut self.path[last_index];
    //     &mut children[*index]
    // }
}

impl<T> Nav for TreeZipper<T> {
    fn child_count(&self) -> usize {
        self.here.children.len()
    }

    fn at_root(&self) -> bool { self.path.is_empty() }

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
    }

    fn seek_child(&mut self, index: usize) {
        let new_index = ChildIndex::compute(self.child_count(), index).unwrap();
    }

    fn to_parent(&mut self) {
        self.path.pop().expect("already at root");
    }

    fn to_root(&mut self) {
        while ! self.at_root() {
            let last_index = self.path.len() - 1;
            let &mut (ref mut children, ref mut index) = &mut self.path[last_index];
            (self.here, *index) = mem::replace(&mut self.path[last_index], (self.here, *index));
            self.to_parent();
        }
    }
}

// impl<T> Zipper for TreeZipper<T> {
//     type Data = T;
//     type Tree = Tree<T>;

//     fn leaf(data: T) -> Self {
//         TreeZipper { here: Tree::leaf(data), path: vec![], }
//     }

//     fn build(mut self) -> Tree<T> {
//         let (mut children, index) = self.path.pop().unwrap();
//         children.remove(index)
//     }

//     // fn stitch<I>(data: T, children: I) -> Self where I: Iterator<Item=Self> {
//     // }

//     // fn asplode(self) -> (T, Vec<Self>) {
//         // (self.data, self.children.into_iter().map(|t| TreeZipper { data: t.data, children: t.children, }).collect())
//     // }

//     fn data(&self) -> &T { &self.here().data }

//     fn data_mut(&mut self) -> &mut T { &mut self.here_mut().data }

//     fn set_data(&mut self, data: T) { self.here_mut().data = data; }

//     fn push_child(&mut self, child: Self) {
//         self.here_mut().children.push(child.build());
//     }

//     fn insert_child(&mut self, index: usize, child: Self) {
//         self.here_mut().children.insert(index, child.build());
//     }
// }



// If we decide to go the TreeViewMut route:
// use std::fmt::Debug;
// use std::ptr;

// struct Tree<T> {
//     data: T,
//     children: Vec<Tree<T>>,
// }

// impl <T> Tree<T> {
//     fn leaf(data: T) -> Self { Tree{data: data, children: vec!(),} }
//     fn new(data: T, children: Vec<Tree<T>>) -> Self {
//         Tree{data: data, children: children,}
//     }
// }

// struct TreePath<'a, T: 'a> {
//     // Currently focused location.
//     root: &'a mut Tree<T>,
//     here_ptr: *mut Tree<T>,
//     history: Vec<(*mut Tree<T>, usize)>,
// }

// impl <'a, T: Debug + 'a> TreePath<'a, T> {
//     fn new(tree: &'a mut Tree<T>) -> Self {
//         println!("Root data: {:?}" , tree . data);
//         let mut path =
//             TreePath{root: tree, here_ptr: ptr::null_mut(), history: vec!(),};
//         println!("Path root data: {:?}" , path . root . data);
//         path.here_ptr = path.root;
//         {
//             let t: &Tree<T> = unsafe { &*path.here_ptr };
//             println!("Here data: {:?}" , t . data);
//         }
//         path
//     }
//     fn here(&self) -> &Tree<T> {
//         {
//             println!("Calling here(), self.root.data is: {:?}" , self . root .
//                      data);
//         }
//         {
//             let t: &Tree<T> = unsafe { &*self.here_ptr };
//             println!("Calling here(), self.here_ptr->data is: {:?}" , t .
//                      data);
//         }
//         unsafe { &*self.here_ptr }
//     }
//     fn here_mut(&mut self) -> &mut Tree<T> { unsafe { &mut *self.here_ptr } }
//     fn data(&self) -> &T { &self.here().data }

//     fn seek_child(&mut self, index: usize) {
//         assert!(index < self . here (  ) . children . len (  ));
//         self.history.push((self.here_ptr, index));
//         self.here_ptr = &mut self.here_mut().children[index];
//     }
// }

// fn main() {
//     let mut tree = Tree::new(1, vec!(Tree:: leaf ( 2 )));
//     let mut path = TreePath::new(&mut tree);
//     println!("Starting at node: {:?}" , path . data (  ));
//     path.seek_child(0);
//     println!("Ending at node: {:?}" , path . data (  ));
// }
