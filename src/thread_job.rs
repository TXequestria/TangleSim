use chain_decompose::Tangle as TangleChain;
use fastrand::Rng;
use tangle_base::{NodeId, Tangle as TangleBase};

fn assert_tangle_equal(base: &TangleBase, chain: &TangleChain) {
    assert_eq!(base.nodes_count(), chain.nodes_count());
    for id in 0..base.nodes_count() {
        assert_eq!(
            base.query_node(id).unwrap().weight,
            chain.get_weight(id).unwrap()
        )
    }
    for i in 0..base.nodes_count() {
        for j in 0..=i {
            assert_eq!(base.is_reachable(i, j), chain.is_reachable(i, j))
        }
    }
    for node_id in 0..base.nodes_count() {
        assert_eq!(base.query_cw(node_id), chain.query_cw(node_id));
        assert_eq!(base.query_score(node_id), chain.query_score(node_id))
    }
}

pub(super) fn gen_tangle(rng: &mut Rng) {
    let mut base = TangleBase::default();
    let mut chain = TangleChain::default();

    for _ in 0..300 {
        let tips = base.tips();
        assert_eq!(tips, chain.tips());
        assert_tangle_equal(&base, &chain);
        let weight = rng.u64(1..1024);
        let nodes_count = base.nodes_count();
        for _ in 5..20 {
            let approvees = select_two(&tips, nodes_count, rng);
            base.insert_node(weight, approvees).unwrap();
            chain.insert_node(weight, approvees).unwrap();
        }
    }

    assert_tangle_equal(&base, &chain);
}

fn select_two(tips: &[NodeId], node_count: usize, rng: &mut Rng) -> (NodeId, NodeId) {
    if node_count <= 1 {
        return (0, 0);
    }
    if tips.len() <= 1 {
        let a = tips[0];
        loop {
            let b = rng.usize(0..node_count);
            if a < b {
                return (a, b);
            } else if a > b {
                return (b, a);
            }
        }
    }

    let chosen = rng.choose_multiple(tips, 2);

    (*chosen[0], *chosen[1])
}
