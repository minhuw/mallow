#![feature(portable_simd)]
use clap::Parser;
use core_affinity::{get_core_ids, set_for_current};
use rand::Rng;
use std::sync::Arc;
use std::sync::Barrier;
use std::thread;
use std::time::{Duration, Instant};

mod kernel;
mod report;
mod system;

use kernel::Kernel;
use report::{print_results, BenchmarkConfig, BenchmarkResult, BenchmarkResults};
use system::cpu_info::get_cpu_info;

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum Operation {
    /// Read-only benchmark
    Read,
    /// Write-only benchmark
    Write,
    /// Read and write benchmark
    ReadWrite,
}

#[derive(Parser)]
#[command(author, version, about = "Memory bandwidth benchmark tool")]
struct Args {
    /// Buffer size in MiB (fixed at 128 MiB)
    #[arg(short, long, default_value_t = 128, hide = true)]
    size: usize,

    /// Stride size in bytes (defaults to CPU's cache line size, specify explicitly to override)
    #[arg(long)]
    stride: Option<usize>,

    /// Duration of measurement in seconds
    #[arg(short, long, default_value_t = 10.0)]
    duration: f64,

    /// Operation type (read, write, or readwrite)
    #[arg(short, long, value_enum, default_value_t = Operation::Read)]
    operation: Operation,

    /// Number of warmup iterations
    #[arg(short, long, default_value_t = 5)]
    warmup: usize,

    /// Enable SIMD reads
    #[arg(long)]
    simd: bool,

    /// Enable parallel processing
    #[arg(short, long)]
    parallel: bool,

    /// Number of threads (default: number of logical CPUs)
    #[arg(short, long)]
    threads: Option<usize>,

    /// Output format (text, csv, json)
    #[arg(short, long, default_value = "text")]
    format: String,

    /// Enable core affinity
    #[arg(long)]
    affinity: bool,
}

fn measure_memory_bandwidth(config: &BenchmarkConfig) -> (f64, f64, usize) {
    // Convert byte size to number of u32 elements
    let num_elements = config.size / std::mem::size_of::<u32>();
    let barrier = Arc::new(Barrier::new(config.thread_count));
    let start_signal = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let mut handles = vec![];
    for thread_id in 0..config.thread_count {
        let barrier = Arc::clone(&barrier);
        let start_signal = Arc::clone(&start_signal);
        let core_ids = config.core_ids.clone();
        let kernel = config.kernel.clone();
        let config = config.clone();

        let handle = thread::spawn(move || {
            if !core_ids.is_empty() {
                let core_id = core_ids[thread_id % core_ids.len()];
                let _ = set_for_current(core_id);
            }

            // Each thread creates its own buffer
            let mut rng = rand::rng();
            let mut data: Vec<u32> = (0..num_elements).map(|_| rng.random()).collect();

            // Wait for all threads to finish initialization
            barrier.wait();

            // Warmup
            for _ in 0..config.warmup_iterations {
                kernel.run(&mut data, config.stride);
            }

            // Wait for all threads to finish warmup
            barrier.wait();

            // First thread sets the start signal
            if thread_id == 0 {
                start_signal.store(true, std::sync::atomic::Ordering::SeqCst);
            }

            // Wait for start signal
            while !start_signal.load(std::sync::atomic::Ordering::SeqCst) {
                std::hint::spin_loop();
            }

            let thread_start = Instant::now();
            let mut total_sum = 0u64;
            let mut iterations = 0usize;

            while thread_start.elapsed().as_secs_f64() < config.duration_secs {
                total_sum = total_sum.wrapping_add(kernel.run(&mut data, config.stride));
                iterations += 1;
            }

            let thread_elapsed = thread_start.elapsed();

            (total_sum as f64, iterations, thread_elapsed)
        });
        handles.push(handle);
    }

    let results: Vec<(f64, usize, Duration)> =
        handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Use the maximum elapsed time across all threads
    let elapsed = results
        .iter()
        .map(|(_, _, elapsed)| *elapsed)
        .max()
        .unwrap();
    let total_sum: f64 = results.iter().map(|(sum, _, _)| sum).sum();
    let total_iterations: usize = results.iter().map(|(_, iters, _)| *iters).sum();

    // Calculate actual number of elements accessed with stride
    let elements_per_iteration = std::cmp::min(
        config.size / config.cpu_cache_info.l1d_line_size.unwrap_or(64),
        (config.size / config.stride)
            + if config.size % config.stride != 0 {
                1
            } else {
                0
            },
    );

    // Calculate number of unique cache lines accessed
    let cache_line_size = config.cpu_cache_info.l1d_line_size.unwrap_or(64);

    // Each access fetches exactly one cache line, regardless of stride
    let bytes_processed = (elements_per_iteration * cache_line_size * total_iterations) as f64;
    let seconds = elapsed.as_secs_f64();
    let bandwidth = bytes_processed / seconds / (1024.0 * 1024.0 * 1024.0); // Convert to GiB/s

    println!("\nBandwidth Calculation Details:");
    println!("  Cache line size: {} bytes", cache_line_size);
    println!("  Elements per iteration: {}", elements_per_iteration);
    println!("  Total iterations: {}", total_iterations);
    println!(
        "  Total bytes processed: {:.2} GiB",
        bytes_processed / (1024.0 * 1024.0 * 1024.0)
    );
    println!("  Elapsed time: {:.3} seconds", seconds);
    if config.thread_count > 1 {
        println!(
            "  Average iterations per thread: {:.1}",
            total_iterations as f64 / config.thread_count as f64
        );
        for (thread_id, (_, iters, _)) in results.iter().enumerate() {
            println!("    Thread {}: {} iterations", thread_id, iters);
        }
    }
    println!("  Bandwidth: {:.2} GiB/s\n", bandwidth);

    (bandwidth, total_sum, total_iterations)
}

fn main() {
    let args = Args::parse();

    // Get available CPU cores
    let core_ids = get_core_ids().unwrap_or_default();
    let available_cores = core_ids.len();

    // Get CPU cache information
    let cache_info = get_cpu_info();

    // Use cache line size by default, or user-specified stride if provided
    let stride_bytes = args
        .stride
        .unwrap_or_else(|| cache_info.l1d_line_size.unwrap_or(64));

    // Convert byte stride to element stride
    let stride = stride_bytes.div_ceil(std::mem::size_of::<u32>());

    // Print CPU cache information
    println!("CPU Cache Information:");
    if let Some(size) = cache_info.l1d_size_kb {
        println!(
            "L1D Cache: {} KB (line size: {} bytes)",
            size,
            cache_info.l1d_line_size.unwrap_or(0)
        );
        if let (Some(sets), Some(assoc)) = (cache_info.l1d_sets, cache_info.l1d_associativity) {
            println!("         Sets: {}, Associativity: {}-way", sets, assoc);
        }
    }
    if let Some(size) = cache_info.l2_size_kb {
        println!(
            "L2 Cache:  {} KB (line size: {} bytes)",
            size,
            cache_info.l2_line_size.unwrap_or(0)
        );
        if let (Some(sets), Some(assoc)) = (cache_info.l2_sets, cache_info.l2_associativity) {
            println!("         Sets: {}, Associativity: {}-way", sets, assoc);
        }
    }
    if let Some(size) = cache_info.l3_size_kb {
        println!(
            "L3 Cache:  {} KB (line size: {} bytes)",
            size,
            cache_info.l3_line_size.unwrap_or(0)
        );
        if let (Some(sets), Some(assoc)) = (cache_info.l3_sets, cache_info.l3_associativity) {
            println!("         Sets: {}, Associativity: {}-way", sets, assoc);
        }
    }
    println!();

    // Convert MiB to bytes (not number of elements)
    let size = args.size * 1024 * 1024;
    let size_mib = size as f64 / (1024.0 * 1024.0);

    let kernel = match (args.operation, args.simd) {
        (Operation::Read, false) => Kernel::ScalarRead,
        (Operation::Read, true) => Kernel::SimdRead,
        (Operation::Write, false) => Kernel::ScalarWrite,
        (Operation::Write, true) => Kernel::SimdWrite,
        (Operation::ReadWrite, _) => {
            println!("ReadWrite operation not yet implemented");
            std::process::exit(1);
        }
    };

    let thread_count = if args.parallel {
        args.threads.unwrap_or(available_cores)
    } else {
        1
    };

    let config = BenchmarkConfig {
        size,
        stride,
        duration_secs: args.duration,
        warmup_iterations: args.warmup,
        kernel: kernel.clone(),
        thread_count,
        core_ids: if args.affinity { core_ids } else { vec![] },
        cpu_cache_info: cache_info.clone(),
    };

    let mut benchmark_results = BenchmarkResults {
        results: Vec::new(),
        config: config.clone(),
    };

    if args.parallel {
        println!("Using parallel measurement with {:?} kernel", kernel);
    } else {
        println!("Using single-threaded measurement with {:?} kernel", kernel);
    }

    let (bandwidth, _sum, iterations) = measure_memory_bandwidth(&config);

    benchmark_results.results.push(BenchmarkResult {
        size_mib,
        bandwidth_gib_s: bandwidth,
        simd_enabled: matches!(config.kernel, Kernel::SimdRead | Kernel::SimdWrite),
        parallel_enabled: config.thread_count > 1,
        affinity_enabled: !config.core_ids.is_empty(),
        iterations,
        warmup_iterations: config.warmup_iterations,
        threads: config.thread_count,
    });

    print_results(&benchmark_results, &args.format);
}
