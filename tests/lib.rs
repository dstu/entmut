#[macro_use(owned_tree, shared_tree)]
extern crate entmut;

/// Defines macros for generalized tests of Nav impls.
#[macro_use]
mod view_tests;

mod owned {
    view_tests!(owned_tree);
}

mod shared {
    view_tests!(shared_tree);
}
