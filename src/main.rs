mod thread_job;

const THREAD_COUNT:u64 = 16;
const RAND_SEED:u64 = 0x1145141919810;
const MAX_BATCH_INSERT_COUNT:usize = 30;
const INSERTION_ROUNDS:usize = 400;
const MAX_OWN_WEIGHT:u64 = 1024;

fn main() {
    use fastrand::Rng;
    use thread_job::gen_tangle;
    let mut handles = Vec::with_capacity(THREAD_COUNT as usize);

    for i in 0..THREAD_COUNT {
        let h = std::thread::spawn(move || {
            let mut rng = Rng::with_seed(RAND_SEED + i);
            gen_tangle(&mut rng);
        });
        handles.push(h)
    }
    
    for (i,h) in handles.into_iter().enumerate() {
        match h.join() {
            Ok(()) => println!("worker {i} finished successfully!"),
            Err(_) => println!("worker {i} panicked"),
        }
    }
    
}

