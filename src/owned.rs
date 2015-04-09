use ::{Guard, Nav};

pub struct Tree<T> {
    data: T, children: Vec<Tree<T>>,
}

pub struct DataGuard<'a, T: 'a> {
    tree: &'a Tree<T>,
}

impl<'a, T: 'a> Guard<'a, T> for DataGuard<'a, T> {
    fn super_deref<'s>(&'s self) -> &'a T {
        &self.tree.data
    }
}

pub struct Navigator<'a, T: 'a> {
    here: &'a Tree<T>, path: Vec<&'a Tree<T>>,
}

impl<'a, T: 'a> Nav<'a> for Navigator<'a, T> {
    type Data = T;
    type DataGuard = DataGuard<'a, T>;

    fn seek_sibling(&mut self, offset: isize) {
        // TODO
    }

    fn seek_child(&mut self, index: usize) {
        // TODO
    }

    fn child_count(&self) -> usize {
        self.here.children.len()
    }

    fn is_root(&self) -> bool {
        self.path.is_empty()
    }

    fn to_parent(&mut self) {
        self.here = self.path.pop().expect("Already at root");
    }

    fn data(&self) -> DataGuard<'a, T> {
        DataGuard { tree: self.here, }
    }
}
