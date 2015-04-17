use ::{Guard, Nav};
use ::util::{ChildIndex, SiblingIndex};

// use std::iter::Iterator;

pub struct Tree<T> {
    data: Vec<T>, offsets: Vec<usize>, children: Vec<usize>,
}

impl<T> Tree<T> {
    // pub fn new<I: Iterator<Item=(T, I)>>(data: T, children: I) -> Self {
    //     let mut tree = Tree::empty();
    //     tree.data.push(data);
    //     tree.offsets.push(offset);
    //     tree.build(0us, children);
    //     return tree;
    // }

    // fn build<I: Iterator<Item=(T, I)>>(&mut self, node: usize, children: I) {
    //     let mut i = 0us;
    //     for (child_data, child_children) in children {
    //         self.data.push(child_data);
    //         i += 1;
    //         self.children.push(node + i);
    //     }
    //     self.offsets.push(self.offsets[self.offsets.len() - 1] + i);
    // }

    pub fn empty() -> Self {
        Tree { data: vec![], offsets: vec![], children: vec![], }
    }

    // pub fn add_node<I: Iterator<Item=T>>
    //   (&mut self, parent: usize, children: &mut I) {
    //     assert![parent == self.size()];
    //     loop {
    //         let mut child_index = 0us;
    //         let mut child_count = 0us;
    //         match children.next() {
    //             None => break,
    //             Some(c) => {
                    
    //             },
    //         }
    //     }

    // }
    
    pub fn size(&self) -> usize {
        self.data.len()
    }

    fn child_count(&self, index: usize) -> usize {
        match index.checked_add(1) {
            None =>
                panic!["numerical overflow in computing child count"],
            Some(x) if x > self.size() =>
                panic!["no such child {} (only {} nodes in tree)", index, self.size()],
            Some(x) if x == self.size() =>
                self.size() - self.offsets[index],
            Some(x) =>
                self.offsets[x] - self.offsets[index],
        }
    }

    fn child_of(&self, parent: usize, index: usize) -> usize {
        assert![parent < self.size()];
        match self.offsets[parent].checked_add(index) {
            Some(x) => self.children[x],
            None => panic!["numerical overflow in computing child offset"],
        }
    }
}

pub struct DataGuard<'a, T: 'a> {
    tree: &'a Tree<T>, index: usize,
}

impl<'a, T: 'a> Guard<'a, T> for DataGuard<'a, T> {
    fn super_deref<'s>(&'s self) -> &'a T {
        &self.tree.data[self.index]
    }
}

pub struct Navigator<'a, T: 'a> {
    tree: &'a Tree<T>, path: Vec<usize>,
}

impl<'a, T: 'a> Navigator<'a, T> {
    fn index(&self) -> usize {
        self.path[self.path.len() - 1]
    }
}

impl<'a, T: 'a> Nav<'a> for Navigator<'a, T> {
    type Data = T;
    type DataGuard = DataGuard<'a, T>;

    fn seek_sibling(&mut self, offset: isize) {
        let new_index =
            if self.path.len() < 1 {
                SiblingIndex::Root
            } else {
                let parent = self.path[self.path.len() - 1];
                SiblingIndex::compute(self.tree.child_count(parent),
                                      parent,
                                      offset)
            }.unwrap();
        let offset_index = match self.path.pop() {
            Some(parent) =>
                self.tree.child_of(parent, new_index),
            None =>
                panic!["tree corruption"],
        };
        self.path.push(offset_index);
    }

    fn seek_child(&mut self, index: usize) {
        let new_index =
            ChildIndex::compute(self.child_count(), index).unwrap();
        let offset = self.tree.offsets[self.index()];
        let child = self.tree.children[offset + new_index];
        self.path.push(child);
    }

    fn child_count(&self) -> usize {
        self.tree.child_count(self.index())
    }

    fn at_root(&self) -> bool {
        self.path.len() == 1
    }

    fn to_parent(&mut self) {
        assert![self.path.len() <= 1, "Already at root"];
        self.path.pop();
    }

    fn to_root(&mut self) {
        self.path.clear();
        self.path.push(0);
    }

    fn data(&self) -> DataGuard<'a, T> {
        DataGuard { tree: self.tree, index: self.index(), }
    }
}
