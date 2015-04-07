use std::rc::Rc;
use std::cell::{Ref, RefCell, RefMut};
use std::ops::Deref;

pub struct TreeInternal<T> {
    data: RefCell<T>,
    children: RefCell<Vec<Tree<T>>>,
}

pub struct Tree<T>(Rc<TreeInternal<T>>);

impl<T> Tree<T> {
    fn new(data: T, children: Vec<Tree<T>>) -> Self {
        Tree(Rc::new(TreeInternal { data: RefCell::new(data),
                                    children: RefCell::new(children), }))
    }
    
    pub fn leaf(data: T) -> Self {
        Tree::new(data, vec![])
    }

    pub fn data<'s>(&'s self) -> Ref<'s, T> {
        self.data.borrow()
    }

    pub fn data_mut<'s>(&'s mut self) -> RefMut<'s, T> {
        self.data.borrow_mut()
    }

    pub fn children<'s>(&'s self) -> Ref<'s, Vec<Tree<T>>> {
        self.children.borrow()
    }

    pub fn children_mut<'s>(&'s mut self) -> RefMut<'s, Vec<Tree<T>>> {
        self.children.borrow_mut()
    }

    pub fn remove_child(&mut self, index: usize) {
        self.children_mut().remove(index);
    }

    pub fn insert_child(&mut self, index: usize, child: Tree<T>) {
        self.children_mut().insert(index, child);
    }

    pub fn push_child(&mut self, child: Tree<T>) {
        self.children_mut().push(child)
    }
}

impl<T> Deref for Tree<T> {
    type Target = TreeInternal<T>;
    fn deref<'a>(&'a self) -> &'a <Self as Deref>::Target {
        let &Tree(ref s) = self;
        s
    }
}
