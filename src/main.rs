mod thread_job;

const THREAD_COUNT:u64 = 16;
const RAND_SEED:u64 = 0x1145141919810;
const MAX_BATCH_INSERT_COUNT:usize = 30;
const INSERTION_ROUNDS:usize = 500;
const MAX_OWN_WEIGHT:u64 = 1024;

use fastrand::Rng;
use thread_job::{update_tangle_round,QueryTimeBase,QueryTimeChain,TangleBase,TangleChain};
use std::thread::spawn;
use std::sync::mpsc::channel;

use crate::thread_job::QueryTime;

fn main() {
    let (tx,rx) = channel::<(QueryTimeBase,QueryTimeChain)>();
    let mut handles = Vec::with_capacity(THREAD_COUNT as _);
    for i in 0..THREAD_COUNT {
        let tx = tx.clone();
        let h = spawn(move || {
            let mut rng = Rng::with_seed(RAND_SEED + i);
            let mut tangle_base = TangleBase::default();
            let mut tangle_chain = TangleChain::default();
            for _ in 0..INSERTION_ROUNDS {
                let (time_base,time_chain) = update_tangle_round(&mut tangle_base, &mut tangle_chain, &mut rng);
                tx.send((time_base,time_chain)).unwrap()
            }
        });
        handles.push(h);
    }
    drop(tx);
    let mut data_base = Vec::with_capacity(INSERTION_ROUNDS);
    let mut data_chain = Vec::with_capacity(INSERTION_ROUNDS);
    while let Ok((time_base,time_chain)) = rx.recv() {
        data_base.push(time_base);
        data_chain.push(time_chain);
    }
    std::fs::write("data_base.csv", QueryTimeBase::into_csv(data_base.as_slice())).unwrap();
    std::fs::write("data_chain.csv",QueryTime::into_csv(data_chain.as_slice())).unwrap();
    for h in handles {
        h.join().unwrap();
    }
}

