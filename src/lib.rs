use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::{Debug, Error, Formatter};
use std::num::{Int, SignedInt};

pub struct Tree<T> {
    data: T,
    children: Vec<Tree<T>>,
}

struct TraversalCell<'a, T: 'a> {
    tree: &'a Tree<T>,
    index: usize,
}

pub struct Navigator<'a, T: 'a> {
    here: &'a Tree<T>,
    path: Vec<TraversalCell<'a, T>>,
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
        self.children.as_slice()
    }

    pub fn children_mut<'s>(&'s mut self) -> &'s mut [Tree<T>] {
        self.children.as_mut_slice()
    }
}

#[macro_export]
macro_rules! tree {
    ($data:expr) => ($crate::Tree::leaf($data));
    ($data:expr, [$($first:tt)*] $(,[$($rest:tt)*])*) =>
        ($crate::Tree { data: $data,
                        children: vec![tree![$($first)*]
                                       $(,tree![$($rest)*])*] });
}

// impl<T: Show> Show for Tree<T> {
//     fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
//         enum Walk<T> { Down(T), Up, };
//         let mut stack = Vec::new();
//         try![write!(f, "({}", self.data)];
//         stack.push(Walk::Up);
//         for c in self.children.iter().rev() {
//             stack.push(Walk::Down(c));
//         }
//         loop {
//             match stack.pop() {
//                 None => return Ok(()),
//                 Some(Walk::Up) => try![write!(f, ")")],
//                 Some(Walk::Down(t)) => {
//                     try![write!(f, " ({}", t.data)];
//                     stack.push(Walk::Up);
//                     for c in t.children.iter().rev() {
//                         stack.push(Walk::Down(c));
//                     }
//                 },
//             }
//         }
//     }
// }

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
        loop {
            match self.path.pop() {
                None => return,
                Some(traversal) => {
                    self.here = &traversal.tree.children[traversal.index]
                },
            }
        }
    }

    pub fn tree<'s>(&'s self) -> &'s Tree<T> {
        &self.here
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
                    .checked_sub(1).expect("index undeflow")
                    .checked_sub((offset_abs + 1isize).abs() as usize).expect("index underflow")
            } else {
                if offset < 0 {
                    cell.index.checked_sub(offset_abs as usize).expect("index underflow")
                } else {
                    cell.index.checked_add(offset_abs as usize).expect("index overflow")
                }
            };
        assert![new_index < cell.tree.children.len(),
                "sibling index {} out of range (only {} siblings)",
                new_index, cell.tree.children.len()];
        self.here = &cell.tree.children[new_index];
        cell.index = new_index;
        self.path.push(cell);
    }

    pub fn seek_child(&mut self, child_index: usize) {
        assert![child_index < self.here.children.len(),
                "child index {} out of range (only {} children)",
                child_index, self.here.children.len()];
        self.path.push(TraversalCell { tree: self.here,
                                       index: child_index });
        self.here = &self.here.children[child_index];
    }
}

impl<'a, T: 'a> Borrow<Tree<T>> for Navigator<'a, T> {
    fn borrow(&self) -> &Tree<T> {
        self.here
    }
}
