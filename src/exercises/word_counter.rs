use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

fn count_words(text: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        // Normalize: lowercase, strip punctuation
        let word = word
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if !word.is_empty() {
            *counts.entry(word).or_insert(0) += 1;
        }
    }
    counts
}

fn merge_into(target: &mut HashMap<String, usize>, source: HashMap<String, usize>) {
    for (word, count) in source {
        *target.entry(word).or_insert(0) += count;
    }
}

pub fn run() {
    let chunks: Vec<String> = vec![
        "the quick brown fox jumps over the lazy dog the fox the dog".to_string(),
        "rust is a systems programming language rust is fast and safe".to_string(),
        "the dog and the fox are fast the quick fox is quick".to_string(),
        "safe systems are fast systems rust programming is fun".to_string(),
    ];

    println!("Input: {} text chunks", chunks.len());

    // Shared result map
    let result: Arc<Mutex<HashMap<String, usize>>> = Arc::new(Mutex::new(HashMap::new()));

    // Spawn one thread per chunk
    let handles: Vec<_> = chunks
        .into_iter()
        .enumerate()
        .map(|(i, chunk)| {
            let result = Arc::clone(&result);
            thread::spawn(move || {
                let local_counts = count_words(&chunk);
                println!(
                    "  Thread {} counted {} unique words",
                    i,
                    local_counts.len()
                );

                // Merge local results into shared map
                let mut global = result.lock().unwrap();
                merge_into(&mut global, local_counts);
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Extract and sort results
    let counts = result.lock().unwrap();
    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

    println!("\nTop 10 most frequent words:");
    for (i, (word, count)) in sorted.iter().take(10).enumerate() {
        println!("  {}. \"{}\" — {} occurrences", i + 1, word, count);
    }
}
