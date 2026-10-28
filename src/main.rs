mod thread_job;
fn main() {
    use fastrand::Rng;
    use thread_job::gen_tangle;
    let mut handles = Vec::with_capacity(16);

    for i in 0..16 {
        let h = std::thread::spawn(move || {
            let mut rng = Rng::with_seed(1145141919810 + i);
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

