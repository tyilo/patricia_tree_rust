use duplicate::duplicate_item;
use replace_with::replace_with_or_abort;
use std::mem;

#[derive(Debug)]
struct LeafNode<V> {
    key: u64,
    value: V,
}

#[derive(Debug)]
struct InternalNode<V> {
    key_prefix: u64,
    branch_bit: u8,
    left: Box<Node<V>>,
    right: Box<Node<V>>,
}

#[derive(Debug)]
enum Node<V> {
    Leaf(LeafNode<V>),
    Internal(InternalNode<V>),
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
                Node::Internal(InternalNode {
                    key_prefix,
                    branch_bit,
                    ..
                }) if *key_prefix != PatriciaTreeMap::<V>::get_prefix(key, *branch_bit) => node,
                Node::Internal(InternalNode {
                    branch_bit,
                    right,
                    left,
                    ..
                }) => {
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
            Some(Node::Leaf(LeafNode { key: k, value: v })) if k == &key => Some(v),
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

                let leaf = Node::Leaf(LeafNode { key, value });
                replace_with_or_abort(node, |old_node| {
                    let (left, right) = if PatriciaTreeMap::<V>::is_left(key, branch_bit) {
                        (leaf, old_node)
                    } else {
                        (old_node, leaf)
                    };

                    Node::Internal(InternalNode {
                        branch_bit,
                        key_prefix,
                        left: Box::new(left),
                        right: Box::new(right),
                    })
                });

                None
            }

            let node = tree.find_insertion_point_mut(key);
            match node {
                None => {
                    tree.root = Some(Box::new(Node::Leaf(LeafNode { key, value })));
                    None
                }
                Some(node) => match node {
                    Node::Leaf(LeafNode { key: k, value: v }) => {
                        if k != &key {
                            let diff = *k ^ key;
                            do_insert(diff, key, value, node)
                        } else {
                            Some(mem::replace(v, value))
                        }
                    }
                    Node::Internal(InternalNode { key_prefix, .. }) => {
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

pub struct PatriciaTreeMapIterator<'a, V> {
    map: &'a PatriciaTreeMap<V>,
    path: Vec<&'a InternalNode<V>>,
    last_was_left: bool,
}

impl<'a, V> PatriciaTreeMapIterator<'a, V> {
    fn new(map: &'a PatriciaTreeMap<V>) -> Self {
        let path = vec![];
        Self {
            map,
            path,
            last_was_left: true,
        }
    }

    fn find_leftmost(&mut self, node: &'a Node<V>) -> Option<(u64, &'a V)> {
        self.last_was_left = false;
        let mut node = node;
        loop {
            match node {
                Node::Leaf(LeafNode { key, value }) => {
                    break Some((*key, value));
                }
                Node::Internal(internal_node) => {
                    self.path.push(internal_node);
                    self.last_was_left = true;
                    node = &internal_node.left;
                }
            }
        }
    }
}

impl<'a, V> Iterator for PatriciaTreeMapIterator<'a, V> {
    type Item = (u64, &'a V);
    
    fn next(&mut self) -> Option<Self::Item> {
        let prev_parent = self.path.pop();
        match prev_parent {
            None => {
                match &self.map.root {
                    None => None,
                    Some(node) => {
                        if self.last_was_left {
                            self.find_leftmost(node)
                        } else {
                            debug_assert_eq!(self.map.len(), 1);
                            self.last_was_left = true;
                            None
                        }
                    }
                }
            },
            Some(internal_node) => {
                let mut internal_node = internal_node;
                if !self.last_was_left {
                    loop {
                        match self.path.pop() {
                            None => {
                                self.last_was_left = true;
                                return None;
                            }
                            Some(parent_node) => {
                                let is_left = PatriciaTreeMap::<V>::is_left(internal_node.key_prefix, parent_node.branch_bit);
                                internal_node = parent_node;
                                if is_left {
                                    break;
                                }
                            },
                        }
                    }
                }

                self.path.push(internal_node);
                self.find_leftmost(&internal_node.right)
            }
        }
    }
}

impl<V> PatriciaTreeMap<V> {
    pub fn iter(&self) -> PatriciaTreeMapIterator<V> {
        PatriciaTreeMapIterator::new(self)
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
    use std::collections::BTreeMap;
    use std::collections::HashMap;
    use std::hash::Hash;

    #[test]
    fn test_empty_map() {
        let map = PatriciaTreeMap::<String>::new();
        assert_eq!(map.len(), 0);
        assert_eq!(map.get(0), None);
        assert_eq!(map.iter().next(), None);
    }

    #[test]
    fn test_iter() {
        let mut map = PatriciaTreeMap::<&'static str>::new();
        assert_eq!(map.iter().next(), None);

        map.insert(0b001, "B".into());
        let mut iter = map.iter();
        assert_eq!(iter.next(), Some((0b001, &"B")));
        assert_eq!(iter.next(), None);

        map.insert(0b011, "C".into());
        map.insert(0b010, "A".into());
        let mut iter = map.iter();
        assert_eq!(iter.next(), Some((0b010, &"A")));
        assert_eq!(iter.next(), Some((0b001, &"B")));
        assert_eq!(iter.next(), Some((0b011, &"C")));
        assert_eq!(iter.next(), None);
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

    fn from_keys(keys: Vec<u64>) -> (PatriciaTreeMap<String>, BTreeMap<u64, String>) {
        let mut tree = PatriciaTreeMap::<String>::new();
        let mut counter = HashMap::<u64, usize>::new();
        let mut reference = BTreeMap::<u64, String>::new();
        for key in keys.into_iter() {
            let count = counter.entry(key).or_insert(0);
            *count += 1;
            let value = format!("{}-{}", key, count);
            tree.insert(key, value.clone());
            reference.insert(key, value);
        }
        (tree, reference)
    }

    fn test_insertion_impl(keys: Vec<u64>) {
        let (tree, reference) = from_keys(keys);

        assert_eq!(tree.len(), reference.len());

        for (k, v) in reference.into_iter() {
            assert_eq!(tree.get(k), Some(&v));
        }
    }

    fn test_iter_impl(keys: Vec<u64>) {
        let (tree, reference) = from_keys(keys);

        let vec = tree.iter().take(tree.len() + 1).collect::<Vec<_>>();
        assert_eq!(vec.len(), tree.len());

        let map: BTreeMap<_, String> = vec.into_iter().map(|(k, v)| (k, v.clone())).collect();
        assert_eq!(map, reference);
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

        #[test]
        fn test_iter_impl_unique(keys in unique_vec(bits::u64::between(0, 10), 0..100)) {
            test_iter_impl(keys);
        }
    }
}
