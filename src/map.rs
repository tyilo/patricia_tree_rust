use duplicate::duplicate_item;
use replace_with::replace_with_or_abort;
use std::mem;

#[derive(Debug)]
enum Node<V> {
    Leaf {
        key: u64,
        value: V,
    },
    Internal {
        key_prefix: u64,
        branch_bit: u8,
        left: Box<Node<V>>,
        right: Box<Node<V>>,
    },
}

#[derive(Debug)]
pub struct PatriciaTreeMap<V> {
    size: usize,
    root: Option<Box<Node<V>>>,
}

impl<V> PatriciaTreeMap<V> {
    pub fn new() -> Self {
        Self {
            size: 0,
            root: None,
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get_prefix(key: u64, branch_bit: u8) -> u64 {
        let mask = (1 << branch_bit) - 1;
        key & mask
    }

    fn is_left(key: u64, branch_bit: u8) -> bool {
        key & (1 << branch_bit) == 0
    }

    #[duplicate_item(
      method                     reference(type) as_ref(v);
      [find_insertion_point]     [& type]        [v.as_ref()];
      [find_insertion_point_mut] [&mut type]     [v.as_mut()];
    )]
    #[allow(clippy::needless_arbitrary_self_type)]
    fn method(self: reference([Self]), key: u64) -> Option<reference([Node<V>])> {
        fn aux<V>(node: reference([Node<V>]), key: u64) -> reference([Node<V>]) {
            match node {
                Node::Leaf { .. } => node,
                Node::Internal {
                    key_prefix,
                    branch_bit,
                    ..
                } if *key_prefix != PatriciaTreeMap::<V>::get_prefix(key, *branch_bit) => node,
                Node::Internal {
                    branch_bit,
                    right,
                    left,
                    ..
                } => {
                    if PatriciaTreeMap::<V>::is_left(key, *branch_bit) {
                        aux(left, key)
                    } else {
                        aux(right, key)
                    }
                }
            }
        }

        as_ref([self.root]).map(|r| aux(r, key))
    }

    pub fn get(&self, key: u64) -> Option<&V> {
        match self.find_insertion_point(key) {
            Some(Node::Leaf { key: k, value: v }) if k == &key => Some(v),
            _ => None,
        }
    }

    pub fn contains(&self, key: u64) -> bool {
        self.get(key).is_some()
    }

    pub fn insert(&mut self, key: u64, value: V) -> Option<V> {
        fn aux<V>(tree: &mut PatriciaTreeMap<V>, key: u64, value: V) -> Option<V> {
            fn do_insert<V>(diff: u64, key: u64, value: V, node: &mut Node<V>) -> Option<V> {
                let branch_bit = diff.trailing_zeros() as u8;
                let key_prefix = PatriciaTreeMap::<V>::get_prefix(key, branch_bit);

                let leaf = Node::Leaf { key, value };
                replace_with_or_abort(node, |old_node| {
                    let (left, right) = if PatriciaTreeMap::<V>::is_left(key, branch_bit) {
                        (leaf, old_node)
                    } else {
                        (old_node, leaf)
                    };

                    Node::Internal {
                        branch_bit,
                        key_prefix,
                        left: Box::new(left),
                        right: Box::new(right),
                    }
                });

                None
            }

            let node = tree.find_insertion_point_mut(key);
            match node {
                None => {
                    tree.root = Some(Box::new(Node::Leaf { key, value }));
                    None
                }
                Some(node) => match node {
                    Node::Leaf { key: k, value: v } => {
                        if k != &key {
                            let diff = *k ^ key;
                            do_insert(diff, key, value, node)
                        } else {
                            Some(mem::replace(v, value))
                        }
                    }
                    Node::Internal { key_prefix, .. } => {
                        let diff = *key_prefix ^ key;
                        do_insert(diff, key, value, node)
                    }
                },
            }
        }

        let res = aux(self, key, value);
        self.size += res.is_none() as usize;
        res
    }
}

impl<V> Default for PatriciaTreeMap<V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::PatriciaTreeMap;
    use proptest::bits;
    use proptest::collection::hash_set;
    use proptest::collection::vec;
    use proptest::collection::SizeRange;
    use proptest::prelude::*;
    use std::collections::HashSet;
    use std::hash::Hash;

    #[test]
    fn test_empty_map() {
        let map = PatriciaTreeMap::<String>::new();
        assert_eq!(map.len(), 0);
        assert_eq!(map.get(0), None);
    }

    #[test]
    fn test_insert_return_value() {
        let mut map = PatriciaTreeMap::<String>::new();
        assert_eq!(map.get(123), None);
        assert_eq!(map.insert(123, "A".into()), None);
        assert_eq!(map.get(123), Some(&"A".into()));
        assert_eq!(map.insert(123, "B".into()), Some("A".into()));
        assert_eq!(map.get(123), Some(&"B".into()));
    }

    fn unique_vec<T>(element: T, size: impl Into<SizeRange>) -> impl Strategy<Value = Vec<T::Value>>
    where
        T: Strategy,
        T::Value: Hash + Eq,
    {
        let x = hash_set(element, size);
        x.prop_map(|v| v.into_iter().collect())
    }

    fn test_insertion_impl(keys: Vec<u64>) {
        let tree = {
            let mut tree = PatriciaTreeMap::<String>::new();
            for v in keys.iter() {
                tree.insert(*v, format!("{}", *v));
            }
            tree
        };

        let unique_keys = keys.into_iter().collect::<HashSet<u64>>();

        assert_eq!(tree.len(), unique_keys.len());

        for v in unique_keys.iter() {
            assert_eq!(tree.get(*v), Some(&format!("{}", *v)));
        }
    }

    proptest! {
        #[test]
        fn test_insert_with_duplicates(keys in vec(bits::u64::between(0, 10), 0..100)) {
            test_insertion_impl(keys)
        }

        #[test]
        fn test_insert_unique(keys in unique_vec(bits::u64::between(0, 10), 0..100)) {
            test_insertion_impl(keys)
        }
    }
}
