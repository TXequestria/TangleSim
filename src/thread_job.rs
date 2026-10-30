use super::{MAX_BATCH_INSERT_COUNT, MAX_OWN_WEIGHT};
pub(super) use chain_decompose::Tangle as TangleChain;
use fastrand::Rng;
use std::time::Instant;
pub(super) use tangle_base::{Tangle as TangleBase};
use tangle_base::NodeId;
const _: () = const {
    assert!(
        MAX_BATCH_INSERT_COUNT > 5,
        "MAX_BATCH_INSERT_COUNT must be greater than 5"
    );
    assert!(MAX_OWN_WEIGHT > 1, "MAX_OWN_WEIGHT must be greater than 1");
};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub(super) struct QueryTime {
    pub(super) nodes_count: usize,
    pub(super) avg_cw_query: f64,
    pub(super) std_cw_query: f64,
    pub(super) avg_score_query: f64,
    pub(super) std_score_query: f64,
    pub(super) batch_insert_count: usize,
    pub(super) avg_batch_insert: f64,
    pub(super) std_batch_insert: f64,
    _pri: (),
}

impl QueryTime {
    fn new(
        nodes_count: usize,
        cw_query: Vec<f64>,
        score_query: Vec<f64>,
        batch_insert: Vec<f64>,
    ) -> Self {
        let (avg_cw_query, std_cw_query) = get_mean_std(cw_query);
        let (avg_score_query, std_score_query) = get_mean_std(score_query);
        let batch_insert_count = batch_insert.len();
        let (avg_batch_insert, std_batch_insert) = get_mean_std(batch_insert);
        Self {
            nodes_count,
            avg_cw_query,
            std_cw_query,
            avg_score_query,
            std_score_query,
            batch_insert_count,
            avg_batch_insert,
            std_batch_insert,
            _pri: (),
        }
    }
    pub(super) fn into_csv(data: &[Self]) -> String {
        use std::fmt::Write;
        let mut string = format!(
            "nodes_count,avg_cw_query,std_cw_query,avg_score_query,std_score_query,batch_insert_count,avg_batch_insert,std_batch_insert\n"
        );
        for data in data {
            write!(
                &mut string,
                "{},{},{},{},{},{},{},{}\n",
                data.nodes_count,
                data.avg_cw_query,
                data.std_cw_query,
                data.avg_score_query,
                data.std_score_query,
                data.batch_insert_count,
                data.avg_batch_insert,
                data.std_batch_insert,
            )
            .unwrap()
        }
        string
    }
}

pub(super) type QueryTimeBase = QueryTime;
pub(super) type QueryTimeChain = QueryTime;

pub(super) fn update_tangle_round(
    base: &mut TangleBase,
    chain: &mut TangleChain,
    rng: &mut Rng,
) -> (QueryTimeBase, QueryTimeChain) {
    assert_eq!(base.nodes_count(), chain.nodes_count());
    let tips = base.tips();
    assert_eq!(tips, chain.tips());
    let node_count = base.nodes_count();
    let batch_insert_count = rng.usize(5..MAX_BATCH_INSERT_COUNT);
    let mut batch_insert_time_base: Vec<f64> = Vec::with_capacity(batch_insert_count);
    let mut batch_insert_time_chain: Vec<f64> = Vec::with_capacity(batch_insert_count);

    for _ in 0..batch_insert_count {
        let weight = rng.u64(1..MAX_OWN_WEIGHT);
        let target = select_two(&tips, node_count, rng);
        let now = Instant::now();
        base.insert_node(weight, target).unwrap();
        let time = now.elapsed();
        batch_insert_time_base.push(time.as_nanos() as _);
        let now = Instant::now();
        chain.insert_node(weight, target).unwrap();
        let time = now.elapsed();
        batch_insert_time_chain.push(time.as_nanos() as _);
    }

    assert_eq!(base.nodes_count(), chain.nodes_count());
    let node_count = base.nodes_count();
    let mut cw_query_base: Vec<f64> = Vec::with_capacity(node_count);
    let mut score_query_base: Vec<f64> = Vec::with_capacity(node_count);

    let mut cw_query_chain: Vec<f64> = Vec::with_capacity(node_count);
    let mut score_query_chain: Vec<f64> = Vec::with_capacity(node_count);

    for id in 0..base.nodes_count() {
        let now = Instant::now();
        let cw_base = base.query_cw(id).unwrap();
        let time = now.elapsed();
        cw_query_base.push(time.as_nanos() as _);
        let now = Instant::now();
        let cw_chain = chain.query_cw(id).unwrap();
        let time = now.elapsed();
        cw_query_chain.push(time.as_nanos() as _);
        assert_eq!(cw_chain, cw_base);
        let now = Instant::now();
        let score_base = base.query_score(id).unwrap();
        let time = now.elapsed();
        score_query_base.push(time.as_nanos() as _);
        let now = Instant::now();
        let score_chain = chain.query_score(id).unwrap();
        let time = now.elapsed();
        score_query_chain.push(time.as_nanos() as _);
        assert_eq!(score_base, score_chain);
    }

    (
        QueryTimeBase::new(
            node_count,
            cw_query_base,
            score_query_base,
            batch_insert_time_base,
        ),
        QueryTimeChain::new(
            node_count,
            cw_query_chain,
            score_query_chain,
            batch_insert_time_chain,
        ),
    )
}

type Mean = f64;
type Std = f64;

fn get_mean_std(data: Vec<f64>) -> (Mean, Std) {
    assert_ne!(data.len(), 0);
    let len = data.len();
    let mean: f64 = data.iter().copied().sum();
    let mean = mean / data.len() as f64;

    let std: f64 = data.into_iter().map(|x| (x - mean).powi(2)).sum();
    let std = std / len as f64;
    let std = std.sqrt();

    (mean, std)
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
