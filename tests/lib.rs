#[macro_use(owned_tree, shared_tree)]
extern crate entmut;

// This will define macros for generalized tests of Nav and Editor impls.
#[cfg(test)]
#[macro_use]
mod view_tests;

#[cfg(test)]
#[macro_use]
mod owned {
    view_tests!(owned_tree);
}

#[cfg(test)]
#[macro_use]
mod shared {
    view_tests!(shared_tree);
}
