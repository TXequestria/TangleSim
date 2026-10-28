use std::num::NonZeroU64;
use crate::tangle::NodeId;
pub type Weight = NonZeroU64;
#[derive(Clone,Debug)]
pub(super) struct NodeInfo {
    own_weight: Weight,
    approvees: Option<(NodeId,NodeId)>,
    approvors: Vec<NodeId>,
}

impl NodeInfo {
    pub(super) const fn genesis() -> Self {
        let unit = unsafe { Weight::new_unchecked(1) };
        Self::new(unit,None)
    }
    pub(super) const fn new(own_weight: Weight, approvees: Option<(NodeId, NodeId)>) -> Self {
        Self {
            own_weight,
            approvees,
            approvors: Vec::new(),
        }
    }
    pub(super) fn add_approvor(&mut self, approvor:NodeId) {
        self.approvors.push(approvor)
    }
    pub(super) const fn get_approvees(&self) -> Option<&(NodeId,NodeId)> {
        (&self.approvees).as_ref()
    }
    pub(super) const fn get_approvors(&self) -> &[NodeId] {
        self.approvors.as_slice()
    }
    pub(super) const fn get_weight(&self) -> Weight {
        self.own_weight
    }
    #[cfg(test)]
    pub(super) fn assert_local_valid(&self,id:NodeId) {
        let mut prev = None;
        for approvor in &self.approvors {
            assert!(*approvor > id);
            if let Some(prev) = prev {
                assert!(*approvor > prev)
            }
            prev = Some(*approvor);
        }
        match self.approvees {
            None => {},
            Some((a,b)) => {
                assert!(a < id && b < id)
            },
        }
    }
    #[cfg(test)]
    pub(super) const fn is_approving(&self,id:NodeId) -> bool {
        match self.approvees {
            None => false,
            Some((a,b)) => a == id || b == id
        }
    }
    #[cfg(test)]
    pub(super) fn is_approved_by(&self,id:NodeId) -> bool {
        self.approvors.binary_search(&id).is_ok()
    }
}
