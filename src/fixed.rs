use std::ops::Index;

struct TreeInternal<T> {
    data: Vec<T>,
    child_lengths: Vec<usize>,
    children: Vec<usize>,
}

pub struct Tree<'a, T: 'a> {
    tree: &'a TreeInternal<T>,
    here: usize,
}

pub struct Children<'a, T: 'a> {
    tree: &'a TreeInternal<T>,
    here: usize,
}

impl<'a, T> Tree<'a, T> {
    pub fn data<'s>(&'s self) -> &'s T {
        &self.tree.data[self.here]
    }

    pub fn children<'s>(&'s self) -> Children<'s, T> {
        Children { tree: self.tree, here: self.here, }
    }
}

impl<'a, T> Index<usize> for Tree<'a, T> {
    type Output = Tree<'a, T>;

    fn index<'s>(&'s self, index: usize) -> &'s Tree<'a, T> {
        assert![index < self.tree.child_lengths[self.here]];
        Tree { tree: self.tree, here: self.tree.children[self.here + index] }
    }
}
