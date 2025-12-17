//! Performance benchmark runner for lazynet.
//!
//! Prerequisites: HTTP server running on localhost:8080
//! Run with: cargo run --bin bench_runner --release

use _lazynet::Lazynet;
use std::time::Instant;

const URL: &str = "http://127.0.0.1:8080/";

fn run_benchmark(num_requests: usize, concurrency: usize) -> (usize, usize, std::time::Duration) {
    let start = Instant::now();

    let lazynet = Lazynet::with_config(100, concurrency, 30);

    for _ in 0..num_requests {
        lazynet.send(URL.to_string());
    }
    lazynet.send_end();

    let mut success_count = 0;
    let mut error_count = 0;

    while let Some(response) = lazynet.recv() {
        if response.error.is_some() {
            error_count += 1;
        } else {
            success_count += 1;
        }
    }

    (success_count, error_count, start.elapsed())
}

fn main() {
    println!("=== Lazynet Benchmark ===");
    println!("URL: {}", URL);
    println!();

    // Warmup
    println!("Warming up...");
    let _ = run_benchmark(100, 1000);
    println!();

    // Throughput benchmarks
    let num_cores = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);

    println!("=== Throughput Benchmarks ({} cores) ===", num_cores);
    let mut max_rate = 0.0_f64;
    for num_requests in [100, 500, 1000, 5000, 30000, 100000] {
        let (success, errors, elapsed) = run_benchmark(num_requests, 1000);
        let rate = (success + errors) as f64 / elapsed.as_secs_f64();
        max_rate = max_rate.max(rate);
        println!(
            "  {:6} requests: {:6} ok, {:3} err, {:7.2?}, {:8.0} req/s",
            num_requests, success, errors, elapsed, rate
        );
    }
    println!();

    // Single-core estimate
    let estimated_single_core = max_rate / num_cores as f64;
    println!("=== Single-Core Estimate ===");
    println!("  {:8.0} req/s (max throughput / {} cores)", estimated_single_core, num_cores);
    println!();

    // Concurrency benchmarks
    println!("=== Concurrency Benchmarks (1000 requests) ===");
    for concurrency in [10, 50, 100, 300, 500, 1000] {
        let (success, errors, elapsed) = run_benchmark(1000, concurrency);
        let rate = (success + errors) as f64 / elapsed.as_secs_f64();
        println!(
            "  concurrency {:4}: {:5} ok, {:3} err, {:7.2?}, {:8.0} req/s",
            concurrency, success, errors, elapsed, rate
        );
    }
    println!();

    println!("=== Done ===");
}
