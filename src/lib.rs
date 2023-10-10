const MIN_DATA_SIZE_FOR_PARALLELISM: usize = 1024;

pub fn compute<T, R, F>(f: &'static F, data: Vec<T>) -> Vec<R>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T) -> R,
    F: Sync,
{
    if data.len() < MIN_DATA_SIZE_FOR_PARALLELISM {
        compute_one_thread(f, data)
    } else {
        compute_multiple_threads(f, data)
    }
}

fn compute_one_thread<T, R, F>(f: &F, data: Vec<T>) -> Vec<R>
where
    F: Fn(T) -> R,
{
    let mut result = Vec::with_capacity(data.len());

    for d in data {
        result.push(f(d));
    }

    result
}

fn compute_multiple_threads<T, R, F>(f: &'static F, data: Vec<T>) -> Vec<R>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T) -> R,
    F: Sync,
{
    let data_size = data.len();

    let threads_cnt = std::thread::available_parallelism()
        .map_err(|e| {
            // TODO: log
            println!("failed getting cpu counts: {:#?}. Using 16", e);
            e
        })
        .map(|cnt| cnt.get())
        .unwrap_or(16);

    // TODO: log
    println!("threads count: {threads_cnt}");

    let mut data_chunks = Vec::with_capacity(threads_cnt);

    for _ in 0..threads_cnt {
        data_chunks.push(Vec::with_capacity(data_size / threads_cnt));
    }

    for (i, d) in data.into_iter().enumerate() {
        data_chunks[i % threads_cnt].push((i, d));
    }

    let mut handles = Vec::with_capacity(threads_cnt);

    for chunk in data_chunks {
        handles.push(std::thread::spawn(move || {
            let mut result = Vec::with_capacity(data_size / threads_cnt);

            for (i, d) in chunk {
                result.push((i, f(d)));
            }

            result
        }));
    }

    let mut result_unordered = Vec::with_capacity(threads_cnt);

    for handle in handles {
        result_unordered.extend(handle.join().expect("Failed joining threads"));
    }

    result_unordered.sort_by(|l, r| l.0.cmp(&r.0));

    result_unordered.into_iter().map(|(_, r)| r).collect()
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use super::*;

    fn square(x: u32) -> u32 {
        x*x
    }

    #[test]
    fn test_simple() {
        let mut rng = rand::thread_rng();

        let size = 2_000_000;

        let data = vec![rng.gen_range(2..100); size];


        let start = std::time::Instant::now();
        let mut one_thread_result = Vec::with_capacity(data.len());
        for d in data.iter() {
            one_thread_result.push(d * d);
        }
        let elapsed = start.elapsed().as_secs_f64();
        println!("one thread elapsed: {elapsed}s");

        let start = std::time::Instant::now();
        let result = compute(&square, data);
        let elapsed = start.elapsed().as_secs_f64();
        println!("multiple threads elapsed: {elapsed}s");

        for (r1, r2) in one_thread_result.into_iter().zip(result) {
            assert_eq!(r1, r2);
        }
    }

    fn sleepy_foo(ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    #[test]
    fn test_sleep() {
        let size = 3_600;

        let data = vec![1; size];

        let start = std::time::Instant::now();
        let mut one_thread_result = Vec::with_capacity(data.len());
        for d in data.iter() {
            one_thread_result.push(sleepy_foo(*d));
        }
        let elapsed = start.elapsed().as_secs_f64();
        println!("one thread elapsed: {elapsed}s");

        let start = std::time::Instant::now();
        compute(&sleepy_foo, data);
        let elapsed = start.elapsed().as_secs_f64();
        println!("multiple threads elapsed: {elapsed}s");
    }
}
