use std::fmt::{FormatError, Formatter, Show};

pub struct Tree<T> {
    pub data: T,
    pub children: Vec<Tree<T>>,
}

pub struct Zipper<T> {
    pub here: Tree<T>,
    lefts: Vec<Tree<T>>,
    rights: Vec<Tree<T>>,
    parent_path: Vec<(Vec<Tree<T>>, T, Vec<Tree<T>>)>,
}

pub enum Modified<T> {
    Old(Zipper<T>),
    New(Zipper<T>),
}

impl<T> Modified<T> {
    pub fn is_new(&self) -> bool {
        match *self {
            Modified::New(z) => true,
            _ => false,
        }
    }

    pub fn is_old(&self) -> bool {
        match *self {
            Modified::Old(z) => true,
            _ => false,
        }
    }
    
    pub fn unwrap(self) -> Zipper<T> {
        match self {
            Modified::New(z) => z,
            Modified::Old(z) => z,
        }
    }
}

impl<T> Tree<T> {
    pub fn new(data: T, children: Vec<Tree<T>>) -> Tree<T> {
        Tree {
            data: data,
            children: children,
        }
    }

    pub fn leaf(data: T) -> Tree<T> {
        Tree {
            data: data,
            children: Vec::new(),
        }
    }
    
    pub fn zipper(self) -> Zipper<T> {
        Zipper {
            lefts: Vec::new(),
            here: self,
            rights: Vec::new(),
            parent_path: Vec::new(),
        }
    }
}

impl<T: Show> Show for Tree<T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FormatError> {
        match write!(f, "({}", self.data) {
            Err(e) => return Err(e),
            _ => (),
        }
        for c in self.children.iter() {
            match write!(f, " {}", c) {
                Err(e) => return Err(e),
                _ => (),
            }
        }
        write!(f, ")")
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

impl<T> Zipper<T> {
    pub fn to_root(mut self) -> Zipper<T> {
        let mut lefts = self.lefts;
        let mut here = self.here;
        let mut rights = self.rights;
        loop {
            match self.parent_path.pop() {
                None => return Zipper {
                    lefts: Vec::new(),
                    rights: Vec::new(),
                    here: here,
                    parent_path: Vec::new(),
                },
                Some((parent_lefts, parent_data, parent_rights)) => {
                    let mut new_children = Vec::with_capacity(lefts.len() + rights.len() + 1);
                    new_children.extend(lefts.into_iter());
                    new_children.push(here);
                    new_children.extend(rights.into_iter());
                    here = Tree::new(parent_data, new_children);
                    lefts = parent_lefts;
                    rights = parent_rights;
                },
            }
        }
    }

    pub fn to_left(mut self) -> Modified<T> {
        let sibling = self.lefts.pop();
        if sibling.is_none() {
            Modified::Old(self)
        } else {
            self.rights.push(self.here);
            self.here = sibling.unwrap();
            Modified::New(self)
        }
    }

    pub fn to_right(mut self) -> Modified<T> {
        let sibling = self.rights.pop();
        if sibling.is_none() {
            Modified::Old(self)
        } else {
            self.lefts.push(self.here);
            self.here = sibling.unwrap();
            Modified::New(self)
        }
    }

    pub fn tree(self) -> Tree<T> {
        self.to_root().here
    }

    pub fn to_parent(mut self) -> Modified<T> {
        match self.parent_path.pop() {
            None => Modified::Old(self),
            Some((parent_lefts, parent_data, parent_rights)) => {
                let mut new_children = Vec::with_capacity(self.lefts.len() + self.rights.len() + 1);
                new_children.extend(self.lefts.into_iter());
                new_children.push(self.here);
                new_children.extend(self.rights.into_iter());
                self.lefts = parent_lefts;
                self.here = Tree::new(parent_data, new_children);
                self.rights = parent_rights;
                Modified::New(self)
            },
        }
    }

    pub fn to_first_child(self) -> Modified<T> {
        self.to_child_at(0u)
    }

    pub fn to_child_at(mut self, child_index: uint) -> Modified<T> {
        if self.here.children.len() <= child_index {
            Modified::Old(self)
        } else {
            let child = self.here.children.remove(child_index).unwrap();
            let mut lefts = Vec::with_capacity(child_index);
            let mut rights = Vec::with_capacity(self.here.children.len() - child_index);
            let mut i = self.here.children.into_iter();
            for _ in range(0, child_index) {
                lefts.push(i.next().unwrap());
            }
            for x in i.rev() {
                rights.push(x);
            }
            self.parent_path.push((self.lefts, self.here.data, self.rights));
            self.lefts = lefts;
            self.here = child;
            self.rights = rights;
            Modified::New(self)
        }
    }

    pub fn to_last_child(self) -> Modified<T> {
        if self.here.children.is_empty() {
            Modified::Old(self)
        } else {
            let child_index = self.here.children.len() - 1;
            self.to_child_at(child_index)
        }
    }

    pub fn set_tree(&mut self, here: Tree<T>) {
        self.here = here;
    }

    pub fn shrink_to_fit(&mut self) {
        self.here.children.shrink_to_fit();
        self.lefts.shrink_to_fit();
        self.rights.shrink_to_fit();
        self.parent_path.shrink_to_fit();
    }

    pub fn push_left(&mut self, sibling: Tree<T>) {
        self.lefts.push(sibling);
    }

    pub fn to_push_left(mut self, sibling: Tree<T>) -> Zipper<T> {
        self.rights.push(self.here);
        Zipper {
            lefts: self.lefts,
            here: sibling,
            rights: self.rights,
            parent_path: self.parent_path,
        }
    }

    pub fn push_right(&mut self, sibling: Tree<T>) {
        self.rights.push(sibling);
    }

    pub fn to_push_right(mut self, sibling: Tree<T>) -> Zipper<T> {
        self.lefts.push(self.here);
        Zipper {
            lefts: self.lefts,
            here: sibling,
            rights: self.rights,
            parent_path: self.parent_path,
        }
    }

    pub fn push_child_at(&mut self, index: uint, child: Tree<T>) -> bool {
        if index <= self.here.children.len() {
            self.here.children.insert(index, child);
            true
        } else {
            false
        }
    }

    pub fn to_push_child_at(mut self, index: uint, child: Tree<T>) -> Option<Zipper<T>> {
        if index > self.here.children.len() {
            None
        } else {
            self.parent_path.push((self.lefts, self.here.data, self.rights));
            let mut left_children = Vec::with_capacity(index);
            let mut right_children = Vec::with_capacity(self.here.children.len() - index);
            let mut i = self.here.children.into_iter();
            for _ in range(0, index) {
                left_children.push(i.next().unwrap());
            }
            for x in i {
                right_children.push(x);
            }
            Some(Zipper {
                lefts: left_children,
                here: child,
                rights: right_children,
                parent_path: self.parent_path,
            })
        }
    }

    pub fn drop_left(&mut self) -> bool {
        if self.lefts.is_empty() {
            false
        } else {
            self.lefts.pop();
            true
        }
    }

    pub fn drop_right(&mut self) -> bool {
        if self.rights.is_empty() {
            false
        } else {
            self.rights.pop();
            true
        }
    }

    pub fn make_orphan(&mut self) {
        self.lefts.clear();
        self.rights.clear();
        self.parent_path.clear();
    }

    pub fn make_leaf(&mut self) {
        self.here.children.clear();
    }

    pub fn delete(mut self) -> Option<Zipper<T>> {
        match self.rights.pop() {
            Some(new_here) => {
                self.here = new_here;
                Some(self)
            },
            None => match self.lefts.pop() {
                Some(new_here) => {
                    self.here = new_here;
                    Some(self)
                },
                None => match self.parent_path.pop() {
                    None => None,
                    Some((parent_lefts, parent_data, parent_rights)) => {
                        self.lefts = parent_lefts;
                        self.here = Tree::leaf(parent_data);
                        self.rights = parent_rights;
                        Some(self)
                    },
                },
            },
        }
    }
}

impl<T: Clone> Clone for Zipper<T> {
    fn clone(&self) -> Zipper<T> {
        Zipper {
            here: self.here.clone(),
            lefts: self.lefts.clone(),
            rights: self.rights.clone(),
            parent_path: self.parent_path.clone(),
        }
    }

    fn clone_from(&mut self, source: &Zipper<T> ) {
        self.here = source.here.clone();
        self.lefts = source.lefts.clone();
        self.rights = source.rights.clone();
        self.parent_path = source.parent_path.clone();
    }
}
