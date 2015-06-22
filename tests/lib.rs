#[macro_use(owned_tree)]
extern crate entmut;

use ::entmut::Nav;
use ::entmut::owned::Tree;
use std::borrow::Borrow;

// This will define macros for generalized tests of Nav and Editor impls.
// #[cfg(test)]
// mod template;

#[test]
#[allow(unused_variables)]
fn nav_instantiation() {
    let t = owned_tree!["a"];
    let v = t.view();
}

#[test]
fn nav_preserves_leaf_topology() {
    let t = owned_tree!["a"];
    let v = t.view();
    assert![v.at_leaf()];
    assert![v.at_root()];
    assert_eq![0, v.child_count()];
}

// #[test]
// fn nav_preserves_leaf_data() {
//     let t = owned_tree!["a"];
//     let t_data_view = &t.data;
//     let v = t.view();
//     let data: &str = v.borrow();
//     assert_eq!["a", data];
//     assert_eq![t_data_view, data];
// }
