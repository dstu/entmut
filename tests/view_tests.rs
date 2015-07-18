#[macro_export]
macro_rules! view_tests {
    ($tree_macro:ident) => (
        use ::entmut::Nav;

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
        );
}
