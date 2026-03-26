use std::sync::mpsc;
use std::thread;

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }
    let mut i = 5;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 {
            return false;
        }
        i += 6;
    }
    true
}

pub fn run() {
    // Stage 1 → Stage 2 channel (producers → filter)
    let (producer_tx, filter_rx) = mpsc::channel::<u64>();

    // Stage 2 → Stage 3 channel (filter → collector)
    let (filter_tx, collector_rx) = mpsc::channel::<u64>();

    // --- Stage 1: Three producer threads ---
    let ranges: Vec<(u64, u64)> = vec![(1, 100), (100, 200), (200, 300)];

    let producer_handles: Vec<_> = ranges
        .into_iter()
        .enumerate()
        .map(|(i, (start, end))| {
            let tx = producer_tx.clone();
            thread::spawn(move || {
                for n in start..end {
                    tx.send(n).unwrap();
                }
                println!("  Producer {} done (range {}..{})", i, start, end);
            })
        })
        .collect();

    // Drop the original sender so the channel closes when all producers finish
    drop(producer_tx);

    // --- Stage 2: Filter thread (keeps only primes) ---
    let filter_handle = thread::spawn(move || {
        let mut count = 0;
        for n in filter_rx {
            if is_prime(n) {
                filter_tx.send(n).unwrap();
                count += 1;
            }
        }
        println!("  Filter passed {} primes", count);
    });

    // --- Stage 3: Collector thread ---
    let collector_handle = thread::spawn(move || {
        let mut primes: Vec<u64> = collector_rx.iter().collect();
        primes.sort();
        primes
    });

    // Wait for all stages
    for handle in producer_handles {
        handle.join().unwrap();
    }
    filter_handle.join().unwrap();
    let primes = collector_handle.join().unwrap();

    // Print results
    println!("\nFound {} primes in range 1..300:", primes.len());
    for chunk in primes.chunks(10) {
        let line: Vec<String> = chunk.iter().map(|n| format!("{:>4}", n)).collect();
        println!("  {}", line.join(""));
    }
}
