use std::collections::BTreeSet;

use proptest::prelude::*;

fn concat_vec(mut a: Vec<i32>, b: Vec<i32>) -> Vec<i32> {
    a.extend(b);
    a
}

proptest! {
    #[test]
    fn vec_concat_is_associative(a in proptest::collection::vec(any::<i32>(), 0..32),
                                 b in proptest::collection::vec(any::<i32>(), 0..32),
                                 c in proptest::collection::vec(any::<i32>(), 0..32)) {
        let left = concat_vec(concat_vec(a.clone(), b.clone()), c.clone());
        let right = concat_vec(a.clone(), concat_vec(b.clone(), c.clone()));
        prop_assert_eq!(left, right);
    }

    #[test]
    fn vec_empty_is_identity(a in proptest::collection::vec(any::<i32>(), 0..64)) {
        let empty: Vec<i32> = vec![];
        prop_assert_eq!(concat_vec(a.clone(), empty.clone()), a.clone());
        prop_assert_eq!(concat_vec(empty, a.clone()), a);
    }
}

fn union_set(a: &BTreeSet<i32>, b: &BTreeSet<i32>) -> BTreeSet<i32> {
    a.union(b).cloned().collect()
}

proptest! {
    #[test]
    fn set_union_is_associative(a in proptest::collection::btree_set(any::<i32>(), 0..32),
                                b in proptest::collection::btree_set(any::<i32>(), 0..32),
                                c in proptest::collection::btree_set(any::<i32>(), 0..32)) {
        let left = union_set(&union_set(&a, &b), &c);
        let right = union_set(&a, &union_set(&b, &c));
        prop_assert_eq!(left, right);
    }

    #[test]
    fn set_empty_is_identity(a in proptest::collection::btree_set(any::<i32>(), 0..64)) {
        let empty: BTreeSet<i32> = BTreeSet::new();
        prop_assert_eq!(union_set(&a, &empty), a.clone());
        prop_assert_eq!(union_set(&empty, &a), a);
    }
}
