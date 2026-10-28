use super::{ChainId, ChainPos};

#[derive(Debug, Clone, Copy)]
struct NodePos {
    chain_id: ChainId,
    chain_pos: ChainPos,
}

impl PartialEq for NodePos {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.chain_id == other.chain_id
    }
    #[inline]
    fn ne(&self, other: &Self) -> bool {
        self.chain_id != other.chain_id
    }
}

impl Eq for NodePos {}

impl PartialOrd for NodePos {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.chain_id > other.chain_id {
            return Some(std::cmp::Ordering::Greater);
        }
        if self.chain_id == other.chain_id {
            return Some(std::cmp::Ordering::Equal);
        }
        if self.chain_id < other.chain_id {
            return Some(std::cmp::Ordering::Less);
        }
        None
    }
}

impl Ord for NodePos {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.chain_id > other.chain_id {
            return std::cmp::Ordering::Greater;
        }
        if self.chain_id == other.chain_id {
            return std::cmp::Ordering::Equal;
        }
        if self.chain_id < other.chain_id {
            return std::cmp::Ordering::Less;
        }
        unreachable!()
    }
}

pub(super) struct Iter<'a> {
    index:usize,
    base:&'a ReachableIndex
}

impl<'a> Iterator for Iter<'a> {
    type Item = (ChainId,ChainPos);
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let node_pos = self.base.index_list.get(self.index)?;
        self.index += 1;
        Some((node_pos.chain_id,node_pos.chain_pos))
    }
}

#[repr(transparent)]
#[derive(Debug,Clone)]
pub(super) struct ReachableIndex {
    index_list: Vec<NodePos>,
}

impl ReachableIndex {
    #[inline]
    pub(super) const fn iter(&self) -> Iter<'_> {
        Iter {
            index:0,
            base:self
        }
    }
    #[inline]
    pub(super) const fn new() -> Self {
        Self {
            index_list: Vec::new(),
        }
    }
    #[inline]
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            index_list: Vec::with_capacity(capacity),
        }
    }
    #[inline]
    pub(super) fn shrink_to_fit(&mut self) {
        self.index_list.shrink_to_fit();
    }
    #[inline]
    pub(super) fn reserve(&mut self, additional: usize) {
        self.index_list.reserve(additional);
    }
    #[inline]
    pub(super) fn insert(&mut self, node_pos: (ChainId, ChainPos)) {
        let (chain_id, chain_pos) = node_pos;
        let node_pos = NodePos {
            chain_id,
            chain_pos,
        };
        self.insert_inner(node_pos);
    }
    #[inline]
    fn insert_inner(&mut self, node_pos: NodePos) {
        let Some(last_mut) = self.index_list.last_mut() else {
            self.index_list.push(node_pos);
            return;
        };
        if node_pos > *last_mut {
            self.index_list.push(node_pos);
            return;
        }
        if node_pos == *last_mut {
            last_mut.chain_pos = last_mut.chain_pos.max(node_pos.chain_pos);
            return;
        }
        match self.index_list.binary_search(&node_pos) {
            Ok(index) => {
                let to_update = &mut self.index_list[index];
                to_update.chain_pos = to_update.chain_pos.max(node_pos.chain_pos);
            },
            Err(index) => {
                self.index_list.insert(index, node_pos);
            }
        }
    }
    #[inline]
    pub(super) fn merge(&self, other: &Self) -> Self {

        if &raw const *self == &raw const *other {
            return Self {
                index_list:self.index_list.clone()
            };
        }

        let mut p_a = 0usize;
        let mut p_b = 0usize;
        let mut ret = Self::with_capacity(self.index_list.len() + other.index_list.len());
        loop {
            let a = self.index_list.get(p_a);
            let b = other.index_list.get(p_b);
            match (a, b) {
                (Some(&a), Some(&b)) => {
                    if a < b {
                        p_a += 1;
                        ret.insert_inner(a);
                    } else if a == b {
                        p_a += 1;
                        p_b += 1;
                        let chain_id = a.chain_id;
                        let chain_pos = a.chain_pos.max(b.chain_pos);
                        let node_pos = NodePos {
                            chain_id,
                            chain_pos,
                        };
                        ret.insert_inner(node_pos);
                    } else {
                        p_b += 1;
                        ret.insert_inner(b);
                    }
                }
                (Some(&a), None) => {
                    ret.insert_inner(a);
                    p_a += 1;
                }
                (None, Some(&b)) => {
                    ret.insert_inner(b);
                    p_b += 1;
                }
                (None, None) => {
                    return ret;
                }
            }
        }
    }
    #[inline]
    pub(super) fn is_reachable(&self, target_pos: (ChainId, ChainPos)) -> bool {
        let (chain_id, chain_pos) = target_pos;
        let target_pos = NodePos {
            chain_id,
            chain_pos,
        };
        let Ok(index) = self.index_list.binary_search(&target_pos) else {
            return false;
        };

        self.index_list[index].chain_pos >= target_pos.chain_pos
    }
    #[cfg(test)]
    pub(super) fn assert_order(&self) {
        let mut last = None;
        for node in self.index_list.iter() {
            if let Some(last) = last {
                assert!(node > last);
            }
            last = Some(node)
        }
    }
}

impl Default for ReachableIndex {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_merge() {
        let parent_a = [(0, 5), (2, 7), (5, 3), (7, 10), (13, 23)];
        let parent_b = [(0, 9), (1, 4), (5, 2), (10, 3)];
        let mut p_a = ReachableIndex::new();
        for node in parent_a {
            p_a.insert(node);
        }
        let mut p_b = ReachableIndex::new();
        for node in parent_b {
            p_b.insert(node);
        }
        let p_c = p_a.merge(&p_b);
        p_c.assert_order();
        assert!(p_c.is_reachable((0, 5)))
    }

#[test]
fn test_merge_with_empty() {
    let empty = ReachableIndex::new();

    let mut parent = ReachableIndex::new();
    parent.insert((0, 1));

    // 首先确认空索引与非空索引归并
    // 不应该丢失任何信息
    let mut merged = empty.merge(&parent);

    assert_eq!(
        merged.iter().collect::<Vec<_>>(),
        vec![(0, 1)],
        "Merge lost parent's ancestors!"
    );

    // 然后模拟 Node 4 的两个直接父节点插入
    merged.insert((0, 0));
    merged.insert((1, 0));

    assert_eq!(
        merged.iter().collect::<Vec<_>>(),
        vec![(0, 1), (1, 0)],
        "Insert corrupted existing prefixes!"
    );
}

}
