pub type NodeId = usize;
use std::fmt::Display;

use crate::node_info::NodeInfo;
use crate::node_info::Weight;
use nohash::IntSet;

#[derive(Clone, Debug)]
pub struct Tangle {
    nodes: Vec<NodeInfo>,
    tips: IntSet<NodeId>,
}

impl Default for Tangle {
    #[inline]
    fn default() -> Self {
        let mut tips = IntSet::<NodeId>::default();
        tips.insert(0);

        Self {
            nodes: vec![NodeInfo::genesis()],
            tips,
        }
    }
}

impl Tangle {
    #[inline]
    pub fn insert_node(
        &mut self,
        weight: u64,
        approvees: (NodeId, NodeId),
    ) -> Result<NodeId, TangleError> {
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
        let new_id = self.nodes.len();
        let node_info = NodeInfo::new(weight, Some(approvees));
        self.append_node(node_info);
        Ok(new_id)
    }
    #[inline]
    pub const fn nodes_count(&self) -> usize {
        self.nodes.len()
    }
    #[inline]
    pub fn tips(&self) -> Vec<NodeId> {
        let mut ret:Vec<NodeId> = self.tips.iter().copied().collect();
        ret.sort_unstable();
        ret
    }
    #[inline]
    pub fn query_node(&self, id: NodeId) -> Option<NodeData<'_>> {
        let info = self.nodes.get(id)?;
        let approvees = info.get_approvees();
        Some(NodeData {
            weight: info.get_weight(),
            approvees,
            approvors: info.get_approvors(),
            _pri: (),
        })
    }
    #[inline]
    pub fn query_cw(&self, id: NodeId) -> Option<Weight> {
        let base = id;
        let len = self.nodes_count().checked_sub(base)?;
        if len == 0 {
            return None;
        }
        let mut visited = vec![false; len];
        let mut stack = Vec::with_capacity(len);
        let mut weight: u64 = 0;
        stack.push(id);
        visited[0] = true;
        while let Some(id) = stack.pop() {
            let node = &self.nodes[id];
            for &approvor in node.get_approvors() {
                let offset = (approvor)
                    .checked_sub(base)
                    .expect("invariant violated: approvor id less than approvee");
                if !visited[offset] {
                    stack.push(approvor);
                    visited[offset] = true;
                }
            }
            weight = weight
                .checked_add(node.get_weight().get())
                .expect("Cumulative weight overflowed");
        }
        Weight::new(weight)
    }
    #[inline]
    pub fn query_score(&self, id: NodeId) -> Option<Weight> {
        if id >= self.nodes_count() {
            return None;
        }
        let mut visited = vec![false; id + 1];
        let mut stack = Vec::with_capacity(id + 1);
        stack.push(id);
        visited[id] = true;
        let mut score = 0u64;
        while let Some(id) = stack.pop() {
            let node = &self.nodes[id];
            score = score
                .checked_add(node.get_weight().get())
                .expect("Score overflowed");
            let Some(&(a, b)) = node.get_approvees() else {
                continue;
            };
            if !visited[a] {
                visited[a] = true;
                stack.push(a);
            }
            if a == b {
                continue;
            }
            if !visited[b] {
                visited[b] = true;
                stack.push(b)
            }
        }
        Weight::new(score)
    }
    #[inline]
    pub fn is_reachable(&self,src:NodeId,dest:NodeId) -> Option<bool> {
        if src >= self.nodes_count() {
            return None;
        }
        if dest >= self.nodes_count() {
            return None
        }
        if src == dest {
            return Some(true);
        }
        if dest > src {
            return Some(false);
        }

        let mut visited = vec![false;src+1];

        let mut stack = Vec::with_capacity(src + 1);
        stack.push(src);
        visited[src] = true;

        while let Some(id) = stack.pop() {
            if id == dest {
                return Some(true);
            }
            if let Some(&(a,b)) = self.query_node(id).unwrap().approvees {
                if a == dest || b == dest {
                    return Some(true);
                }
                if !visited[a] {
                    stack.push(a);
                    visited[a] = true;
                }
                if b != a && !visited[b] {
                    stack.push(b);
                    visited[b] = true;
                }
            }
        }

        Some(false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeData<'a> {
    pub weight: Weight,
    pub approvees: Option<&'a (NodeId, NodeId)>,
    pub approvors: &'a [NodeId],
    _pri: (),
}

impl Tangle {
    fn append_node(&mut self, node: NodeInfo) {
        let current_node_id = self.nodes.len();
        match node.get_approvees() {
            None => panic!("Cannot append new genesis block!"),
            Some(&(a, b)) => {
                self.tips.remove(&a);
                self.nodes[a].add_approvor(current_node_id);
                if a != b {
                    self.tips.remove(&b);
                    self.nodes[b].add_approvor(current_node_id);
                }
            }
        }
        self.nodes.push(node);
        self.tips.insert(current_node_id);
    }
    #[cfg(test)]
    fn assert_tangle_valid(&self) {
        assert!(self.nodes.len() > 0);
        assert!(self.tips.len() > 0);
        for tip in self.tips.iter() {
            assert!(self.nodes[*tip].get_approvors().is_empty())
        }
        for (id, node) in self.nodes.iter().enumerate() {
            node.assert_local_valid(id);
            let approvors = node.get_approvors();
            if approvors.is_empty() {
                assert!(self.tips.contains(&id))
            } else {
                for approvor_id in approvors {
                    assert!(self.nodes[*approvor_id].is_approving(id))
                }
            }
            match node.get_approvees() {
                None => {
                    assert_eq!(id, 0);
                }
                Some(&(a, b)) => {
                    assert_ne!(id, 0);
                    assert!(self.nodes[a].is_approved_by(id));
                    if a != b {
                        assert!(self.nodes[b].is_approved_by(id))
                    }
                }
            }
        }
    }
}

impl Display for Tangle {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Tangle info:")?;
        writeln!(f, "Node count:{}", self.nodes.len())?;
        write!(f, "Tips: [")?;
        for (index, id) in self.tips.iter().enumerate() {
            if index == 0 {
                write!(f, "{id}")?;
            } else {
                write!(f, ", {id}")?
            }
        }
        write!(f, "]\nContaining Nodes:\n")?;
        for (id, node) in self.nodes.iter().enumerate() {
            writeln!(f, "NodeId = {id}")?;
            writeln!(f, "Own Weight = {}", node.get_weight())?;
            match node.get_approvees() {
                None => writeln!(f, "Genesis")?,
                Some((a, b)) if a == b => {
                    writeln!(f, "Approving: ({a})")?;
                }
                Some((a, b)) if a != b => {
                    writeln!(f, "Approving: ({a},{b})")?;
                }
                _ => unreachable!(),
            }
            writeln!(f, "Approved by: {:?}", node.get_approvors())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TangleError {
    #[error("weight is zero")]
    ZeroWeight,
    #[error("Trying to reference or approve node id {0} which is out of bounds!")]
    NonExistentNode(usize),
}

#[cfg(test)]
mod test {
    use super::Tangle;
    #[test]
    fn test_diamond() {
        let mut tangle = Tangle::default();
        let a = tangle.insert_node(1, (0, 0)).unwrap();
        let b = tangle.insert_node(1, (0, 0)).unwrap();
        let c = tangle.insert_node(1, (1, 2)).unwrap();
        assert!(a == 1 && b == 2 && c == 3);
        tangle.assert_tangle_valid();
        assert_eq!(tangle.query_cw(0).unwrap().get(), 4);
        assert_eq!(tangle.query_cw(1).unwrap().get(), 2)
    }
    #[test]
    fn test_cw_diamond() {
        let mut tangle = Tangle::default();

        tangle.insert_node(1, (0, 0)).unwrap(); // A = 1
        tangle.insert_node(1, (0, 0)).unwrap(); // B = 2
        tangle.insert_node(1, (1, 2)).unwrap(); // C = 3

        assert_eq!(tangle.query_cw(0).unwrap().get(), 4);
        assert_eq!(tangle.query_cw(1).unwrap().get(), 2);
        assert_eq!(tangle.query_cw(2).unwrap().get(), 2);
        assert_eq!(tangle.query_cw(3).unwrap().get(), 1);
    }

    #[test]
    fn test_score_diamond() {
        let mut tangle = Tangle::default();

        let a = tangle.insert_node(1, (0, 0)).unwrap();
        let b = tangle.insert_node(1, (0, 0)).unwrap();
        let c = tangle.insert_node(1, (a, b)).unwrap();

        tangle.assert_tangle_valid();

        assert_eq!(tangle.query_score(0).unwrap().get(), 1);
        assert_eq!(tangle.query_score(a).unwrap().get(), 2);
        assert_eq!(tangle.query_score(b).unwrap().get(), 2);
        assert_eq!(tangle.query_score(c).unwrap().get(), 4);

        // Invalid Node ID
        assert_eq!(tangle.query_score(4), None);
    }

    #[test]
    fn test_score_weighted_diamond() {
        let mut tangle = Tangle::default();

        // Genesis has weight 1.
        let a = tangle.insert_node(2, (0, 0)).unwrap();
        let b = tangle.insert_node(3, (0, 0)).unwrap();
        let c = tangle.insert_node(4, (a, b)).unwrap();

        assert_eq!(tangle.query_score(0).unwrap().get(), 1);
        assert_eq!(tangle.query_score(a).unwrap().get(), 3);
        assert_eq!(tangle.query_score(b).unwrap().get(), 4);
        assert_eq!(tangle.query_score(c).unwrap().get(), 10);

        // Score is immutable after insertion.
        let d = tangle.insert_node(5, (c, c)).unwrap();

        assert_eq!(tangle.query_score(c).unwrap().get(), 10);
        assert_eq!(tangle.query_score(d).unwrap().get(), 15);
    }
}
