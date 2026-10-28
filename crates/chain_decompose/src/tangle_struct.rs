use super::{ChainId, ChainPos, NodeId, reachable_index::ReachableIndex};
use tangle_base::{TangleError, Weight};

#[derive(Debug)]
struct Node {
    weight: Weight,
    chain_id: ChainId,
    chain_pos: ChainPos,
    approvees: ReachableIndex,
}

#[derive(Debug)]
pub struct Tangle {
    nodes: Vec<Node>,
    chains: Vec<Vec<(NodeId, Weight)>>,
    tips: nohash::IntSet<NodeId>,
}

impl Default for Tangle {
    #[inline]
    fn default() -> Self {
        let node = Node {
            weight: Weight::MIN,
            chain_id: 0,
            chain_pos: 0,
            approvees: ReachableIndex::new(),
        };
        let mut tips = nohash::IntSet::default();
        tips.insert(0);
        Self {
            nodes: vec![node],
            chains: vec![vec![(0, Weight::MIN)]],
            tips,
        }
    }
}

impl Tangle {
    #[inline]
    pub fn get_weight(&self, node_id: NodeId) -> Option<Weight> {
        self.nodes.get(node_id).map(|n| n.weight)
    }
    #[inline]
    pub fn is_reachable(&self, src: NodeId, dest: NodeId) -> Option<bool> {
        if src >= self.nodes.len() {
            return None;
        }
        if dest >= self.nodes.len() {
            return None;
        }
        if dest > src {
            return Some(false);
        }
        if dest == src {
            return Some(true);
        }
        Some(self.nodes[src].approvees.is_reachable(self.get_pos(dest)))
    }
    #[inline]
    pub fn tips(&self) -> Vec<NodeId> {
        let mut ret:Vec<NodeId> = self.tips.iter().copied().collect();
        ret.sort_unstable();
        ret
    }
    #[inline]
    pub const fn nodes_count(&self) -> usize {
        self.nodes.len()
    }
    #[inline]
    pub fn insert_node(
        &mut self,
        weight: u64,
        approvees: (NodeId, NodeId),
    ) -> Result<(), TangleError> {
        let (a, b) = approvees;
        if a >= self.nodes.len() {
            return Err(TangleError::NonExistentNode(a));
        }
        if b >= self.nodes.len() {
            return Err(TangleError::NonExistentNode(b));
        }
        let Some(weight) = Weight::new(weight) else {
            return Err(TangleError::ZeroWeight);
        };

        let (mut approvees, (first_node_id, second_node_id)) = {
            let parent_a = &self.nodes[a];
            let parent_b = &self.nodes[b];
            (
                parent_a.approvees.merge(&parent_b.approvees),
                if parent_a.chain_id < parent_b.chain_id {
                    (a, b)
                } else {
                    (b, a)
                },
            )
        };

        let new_node_id = self.nodes.len();
        let first_pos = self.get_pos(first_node_id);
        let second_pos = self.get_pos(second_node_id);

        let mut node_pos = None;

        if first_node_id != second_node_id {
            approvees.reserve(2);
        }
        approvees.insert(first_pos);
        if first_node_id != second_node_id {
            approvees.insert(second_pos);
        }
        approvees.shrink_to_fit();

        for (approvee_chain_id, approvee_chain_pos) in approvees.iter() {
            if self.chains[approvee_chain_id].len() == approvee_chain_pos + 1 {
                let chain_id = approvee_chain_id;
                let chain_pos = approvee_chain_pos + 1;
                node_pos = Some((chain_id, chain_pos));
                self.chains[chain_id].push((new_node_id, weight));
                break;
            }
        }

        let (chain_id, chain_pos) = node_pos.unwrap_or_else(|| {
            let to_ret = (self.chains.len(), 0);
            self.chains.push(vec![(new_node_id, weight)]);
            to_ret
        });

        for (approvee_chain_id, approvee_chain_pos) in approvees.iter() {
            if approvee_chain_id == chain_id {
                continue;
            }
            let tuple = &mut self.chains[approvee_chain_id][approvee_chain_pos];
            tuple.1 = tuple
                .1
                .checked_add(weight.get())
                .expect("Cumulative weight overflowed");
        }

        let node = Node {
            weight,
            chain_id,
            chain_pos,
            approvees,
        };
        self.nodes.push(node);

        self.tips.remove(&first_node_id);
        if first_node_id != second_node_id {
            self.tips.remove(&second_node_id);
        }
        self.tips.insert(new_node_id);
        Ok(())
    }
    #[inline]
    pub fn query_cw(&self,node_id: NodeId) -> Option<Weight> {
        let node = self.nodes.get(node_id)?;
        let chain_id = node.chain_id;
        let chain_pos = node.chain_pos;
        let mut cw = 0u64;

        for &(_,weight) in &self.chains[chain_id][chain_pos..] {
            cw = cw.checked_add(weight.get()).expect("Cumulative weight overflowed");
        }

        Weight::new(cw)
    }
    #[inline]
    pub fn query_score(&self,node_id:NodeId) -> Option<Weight> {
        let node = self.nodes.get(node_id)?;
        let mut weight = node.weight;
        let approvee = &node.approvees;
        for (approvee_chain_id,approvee_chain_index) in approvee.iter() {
            for &(approvee_node_id,_) in &self.chains[approvee_chain_id][..=approvee_chain_index] {
                let approvee = &self.nodes[approvee_node_id];
                weight = weight.checked_add(approvee.weight.get()).expect("Score overflowed")
            }
        }
        Some(weight)
    }
    #[inline]
    fn get_pos(&self, node_id: NodeId) -> (ChainId, ChainPos) {
        let node = &self.nodes[node_id];
        let (chain_id, chain_pos) = (node.chain_id, node.chain_pos);
        (chain_id, chain_pos)
    }
    #[cfg(test)]
    #[inline]
    fn assert_tangle_valid(&self) {
        for (node_id,node) in self.nodes.iter().enumerate() {
            node.approvees.assert_order();
            assert_eq!(self.chains[node.chain_id][node.chain_pos].0,node_id)
        }
        let mut seen = vec![false;self.nodes_count()];
        for chain in self.chains.iter(){
            for &(node_id,_) in chain.iter() {
                assert!(!seen[node_id]);
                seen[node_id] = true;
            }
        }
        assert!(seen.iter().all(|x| *x));
        assert!(self.tips.len() >= 1);
        for &tip_id in self.tips.iter() {
            let tip = &self.nodes[tip_id];
            assert_eq!(tip.chain_pos+1,self.chains[tip.chain_id].len())
        }
    }
}

#[cfg(test)]
mod test {
use super::Tangle;
#[test]
fn test_score_and_cw() {
    let mut t = Tangle::default();

    // Weighted diamond
    t.insert_node(2, (0, 0)).unwrap(); // 1
    t.insert_node(3, (0, 0)).unwrap(); // 2
    t.insert_node(5, (1, 2)).unwrap(); // 3

    let expected = [
        // (Score, CW)
        (1, 11),  // Node 0
        (3, 7),   // Node 1
        (4, 8),   // Node 2
        (11, 5),  // Node 3
    ];

    for (id, (score, cw)) in expected.into_iter().enumerate() {
        assert_eq!(t.query_score(id).unwrap().get(), score);
        assert_eq!(t.query_cw(id).unwrap().get(), cw);
    }

    t.assert_tangle_valid();

    // Node 4 approves Node 1, but not Node 3.
    // Thus it can reach only a proper prefix of Chain 0.
    t.insert_node(7, (1, 1)).unwrap(); // 4

    let expected = [
        (1, 18),  // Node 0
        (3, 14),  // Node 1
        (4, 8),   // Node 2
        (11, 5),  // Node 3
        (10, 7),  // Node 4
    ];

    for (id, (score, cw)) in expected.into_iter().enumerate() {
        assert_eq!(
            t.query_score(id).unwrap().get(),
            score,
            "Incorrect Score at node {id}"
        );
        assert_eq!(
            t.query_cw(id).unwrap().get(),
            cw,
            "Incorrect CW at node {id}"
        );
    }

    t.assert_tangle_valid();
}

}