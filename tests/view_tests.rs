#[macro_export]
macro_rules! view_tests {
    ($tree_macro:ident) => (
        use ::entmut::Nav;
        use std::collections::HashMap;
        use std::hash::Hash;
        use std::iter::Iterator;
        use std::ops::Deref;

        #[test]
        #[allow(unused_variables)]
        fn view_instantiation() {
            let t = $tree_macro!["a"];
            let v = t.view();
        }

        #[test]
        fn view_preserves_leaf_topology() {
            let t = $tree_macro!["a"];
            let v = t.view();
            assert![v.at_leaf()];
            assert![v.at_root()];
            assert_eq![0, v.child_count()];
        }

        #[test]
        fn view_preserves_leaf_data() {
            let t = $tree_macro!["a"];
            let v = t.view();
            assert_eq!["a", *v];
        }

        #[test]
        fn view_seek_root_sibling_noop_succeeds() {
            let t = $tree_macro!["a"];
            let mut v = t.view();
            assert![v.seek_sibling(0)];
        }

        #[test]
        fn view_seek_root_sibling_fails() {
            let t = $tree_macro!["a"];
            let mut v = t.view();
            assert![! v.seek_sibling(-1)];
            assert![! v.seek_sibling(1)];
        }

        #[test]
        fn view_counts_children_correctly() {
            let t = $tree_macro!["a", ["b", ["e"], ["f"]], ["c"], ["d"]];
            let mut v = t.view();
            assert_eq![3, v.child_count()];
            assert![v.seek_child(0)];
            assert_eq![2, v.child_count()];
            assert![v.seek_sibling(1)];
            assert_eq![0, v.child_count()];
        }

        #[derive(Debug, Clone, Copy, Eq, PartialEq)]
        enum TraversalState {
            Visited,
            Exhausted,
        }

        // Traversal of `nav`, which should have unique data at each node.
        //
        // When starting from the tree root, traversal order is: parent before
        // child, left sibling before right.
        //
        // When starting from elsewhere, traversal order is: traverse subtree
        // rooted at initial position as though it were rooted there, then
        // recursively move upwards towards the root and traverse each subtree
        // of unvisited nodes as though rooted there.
        //
        // Returns the next data item in the traversal, or None if the tree has
        // been exhausted.
        fn traverse_next<T, N>(nav: &mut N, state: &mut HashMap<T, TraversalState>) -> Option<T>
            where T: Copy + Eq + PartialEq + Hash,  N: Nav + Deref<Target=T> {
                loop {
                    let data = **nav;
                    match state.get(&data).map(|x| *x)  {
                        None => {
                            // Haven't visited this node before. Mark as visited
                            // and return.
                            state.insert(data, TraversalState::Visited);
                            return Some(data)
                        },
                        Some(TraversalState::Exhausted) => {
                            // Visited this node and all children. Move to next
                            // sibling.
                            if nav.seek_sibling(1) {
                                continue
                            }
                            // No more siblings. Move to parent.
                            if nav.to_parent() {
                                continue
                            }
                            // No parent. Must be at root. Terminate.
                            return None
                        },
                        Some(TraversalState::Visited) => {
                            if nav.at_leaf() {
                                // No children. Mark node as exhausted.
                                state.insert(data, TraversalState::Exhausted);
                                continue
                            } else {
                                // Last child has been visited. Mark node as exhausted.
                                let last_child_index = nav.child_count() - 1;
                                if nav.seek_child(last_child_index) && state.contains_key(&**nav) {
                                    assert![nav.to_parent()];
                                    state.insert(data, TraversalState::Exhausted);
                                    continue
                                }
                                assert![nav.to_parent()];
                                // Node has been visited but is not exhausted. Move
                                // to first child.
                                if nav.seek_child(0) {
                                    continue
                                }
                                assert![false];
                            }
                        },
                    }
                }
            }

        // Iterator wrapping traversal_next.
        struct NavIter<N, T> where T: Copy + Eq + PartialEq + Hash,  N: Nav + Deref<Target=T> {
            nav: N,
            state: HashMap<T, TraversalState>,
        }

        impl<N, T> NavIter<N, T>
            where T: Copy + Eq + PartialEq + Hash, N: Nav + Deref<Target=T> {
                fn new(nav: N) -> Self { NavIter { nav: nav, state: HashMap::new(), } }
            }

        impl<N, T> Iterator for NavIter<N, T>
            where T: Copy + Eq + PartialEq + Hash, N: Nav + Deref<Target=T> {
                type Item = T;

                fn next(&mut self) -> Option<T> {
                    traverse_next(&mut self.nav, &mut self.state)
                }
            }

        // Consumes `nav` and iterates through the entire tree in the traversal
        // order defined by `traversal_next`.
        fn traversal_seq<N, T>(nav: N) -> Vec<T>
            where T: Copy + Eq + PartialEq + Hash, N: Nav + Deref<Target=T> {
                NavIter::new(nav).collect()
            }

        #[test]
        fn view_traversal_maintains_tree_order() {
            {
                let t = $tree_macro![1, [2], [3]];
                assert_eq![traversal_seq(t.view()), vec![1, 2, 3]];
            }
            {
                let t = $tree_macro![1, [2, [3]], [4]];
                assert_eq![traversal_seq(t.view()), vec![1, 2, 3, 4]];
            }
            {
                let t = $tree_macro![1, [2, [3, [4]], [5], [6]], [7]];
                assert_eq![traversal_seq(t.view()), vec![1, 2, 3, 4, 5, 6, 7]];
            }
            {
                let t = $tree_macro![1, [2], [3, [4]], [5]];
                assert_eq![traversal_seq(t.view()), vec![1, 2, 3, 4, 5]];
            }
        }

        #[test]
        fn view_nonroot_seek_sibling_noop_succeeds() {
            let t = $tree_macro![1, [2], [3]];
            let mut nav = t.view();
            assert![nav.seek_child(0)];
            assert![nav.seek_sibling(0)];
            assert_eq![traversal_seq(nav), vec![2, 3, 1]];
        }

        #[test]
        fn view_to_root_seeks_root() {
            {
                let t = $tree_macro![1];
                let mut nav = t.view();
                nav.to_root();
                assert_eq![traversal_seq(nav), vec![1]];
            }
            {
                let t = $tree_macro![1, [2], [3], [4]];
                let mut nav = t.view();
                for i in 0..3 {
                    nav.seek_child(i);
                    assert_eq![*nav, i + 2];
                    nav.to_root();
                    assert_eq![traversal_seq(nav.clone()), vec![1, 2, 3, 4]];
                }
            }
            {
                let t = $tree_macro![1, [2], [3], [4, [5], [6, [7]]]];
                let mut nav = t.view();
                assert![nav.seek_child(2)];
                assert![nav.seek_child(1)];
                assert_eq![*nav, 6];
                nav.to_root();
                assert_eq![traversal_seq(nav), vec![1, 2, 3, 4, 5, 6, 7]];
            }
        }

        // TODO: test that seeking invalid child indices returns false.

        // TODO: test seek_first_sibling and seek_last_sibling behaviors.

        // TODO: test at_leaf, at_root in complex trees after arbitrary
        // navigation operations.
        );
}
