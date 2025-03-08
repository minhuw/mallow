#![feature(portable_simd)]
use clap::Parser;
use core_affinity::{get_core_ids, set_for_current};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

mod kernel;
mod report;
mod system;

use kernel::Kernel;
use report::{print_results, BenchmarkConfig, BenchmarkResult, BenchmarkResults};
use system::cpu_info::{get_cpu_info, CacheInfo};

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

#[derive(Clone)]
struct BandwidthOptions {
    size: usize,
    stride: usize,
    duration: f64,
    warmup: usize,
    kernel: Kernel,
    thread_count: usize,
    core_ids: Vec<core_affinity::CoreId>,
    cache_info: CacheInfo,
}

fn measure_memory_bandwidth(opts: BandwidthOptions) -> (f64, f64) {
    let data: Vec<u32> = (0..opts.size).map(|i| i as u32).collect();
    let data = Arc::new(data);
    let core_ids = opts.core_ids.clone(); // Clone here to avoid partial move

    // Single-threaded case
    if opts.thread_count <= 1 {
        let slice = &data[..];

        // Warmup
        for _ in 0..opts.warmup {
            opts.kernel.run(slice, opts.stride);
        }

        let start = Instant::now();
        let mut total_sum = 0u64;
        let mut iterations = 0usize;

        while start.elapsed().as_secs_f64() < opts.duration {
            total_sum = total_sum.wrapping_add(opts.kernel.run(slice, opts.stride));
            iterations += 1;
        }
        let elapsed = start.elapsed();

        // Calculate number of unique cache lines accessed
        let cache_line_size = opts.cache_info.l1d_line_size.unwrap_or(64);

        // Calculate actual number of elements accessed with stride
        let elements_per_iteration = std::cmp::min(
            opts.size / cache_line_size, // Maximum number of cache lines we can access
            (opts.size / opts.stride) + if opts.size % opts.stride != 0 { 1 } else { 0 },
        );

        // Each access fetches exactly one cache line, regardless of stride
        let bytes_processed = (elements_per_iteration * cache_line_size * iterations) as f64;

        let seconds = elapsed.as_secs_f64();
        let bandwidth = bytes_processed / seconds / 1_000_000_000.0;

        (bandwidth, total_sum as f64)
    } else {
        // Multi-threaded case
        let mut handles = vec![];
        for thread_id in 0..opts.thread_count {
            let data = Arc::clone(&data);
            let core_ids = core_ids.clone();
            let kernel = opts.kernel.clone();
            let opts = opts.clone();

            let handle = thread::spawn(move || {
                if !core_ids.is_empty() {
                    let core_id = core_ids[thread_id % core_ids.len()];
                    let _ = set_for_current(core_id);
                }

                let slice = &data[..];

                // Warmup
                for _ in 0..opts.warmup {
                    kernel.run(slice, opts.stride);
                }

                let start = Instant::now();
                let mut total_sum = 0u64;
                let mut iterations = 0usize;

                while start.elapsed().as_secs_f64() < opts.duration {
                    total_sum = total_sum.wrapping_add(kernel.run(slice, opts.stride));
                    iterations += 1;
                }

                (total_sum as f64, iterations)
            });
            handles.push(handle);
        }

        let start = Instant::now();
        let results: Vec<(f64, usize)> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let elapsed = start.elapsed();

        let total_sum: f64 = results.iter().map(|(sum, _)| sum).sum();
        let total_iterations: usize = results.iter().map(|(_, iters)| iters).sum();

        // Calculate actual number of elements accessed with stride
        let elements_per_iteration = std::cmp::min(
            opts.size / opts.cache_info.l1d_line_size.unwrap_or(64), // Maximum number of cache lines we can access
            (opts.size / opts.stride) + if opts.size % opts.stride != 0 { 1 } else { 0 },
        );

        // Calculate number of unique cache lines accessed
        let cache_line_size = opts.cache_info.l1d_line_size.unwrap_or(64);

        // Each access fetches exactly one cache line, regardless of stride
        let bytes_processed = (elements_per_iteration * cache_line_size * total_iterations) as f64;

        let seconds = elapsed.as_secs_f64();
        let bandwidth = bytes_processed / seconds / 1_000_000_000.0;

        (bandwidth, total_sum)
    }
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

    // Convert MiB to number of u32 elements
    let size = args.size * 1024 * 1024 / std::mem::size_of::<u32>();
    let size_mb = (size * std::mem::size_of::<u32>()) as f64 / 1_000_000.0;

    let mut benchmark_results = BenchmarkResults {
        results: Vec::new(),
        config: BenchmarkConfig {
            simd_enabled: args.simd,
            parallel_enabled: args.parallel,
            thread_count: args.threads.unwrap_or(available_cores),
            total_iterations: 0,
            warmup_iterations: args.warmup,
            affinity_enabled: args.affinity,
            core_ids: if args.affinity {
                core_ids.iter().map(|id| id.id).collect()
            } else {
                vec![]
            },
            cpu_cache_info: cache_info.clone(),
        },
    };

    let kernel = if args.simd {
        Kernel::Simd
    } else {
        Kernel::Scalar
    };

    let opts = BandwidthOptions {
        size,
        stride,
        duration: args.duration,
        warmup: args.warmup,
        kernel: kernel.clone(),
        thread_count: if args.parallel {
            args.threads.unwrap_or(available_cores)
        } else {
            1
        },
        core_ids: core_ids.clone(),
        cache_info: cache_info.clone(),
    };

    if args.parallel {
        println!("Using parallel measurement with {:?} kernel", kernel);
    } else {
        println!("Using single-threaded measurement with {:?} kernel", kernel);
    }

    let (bandwidth, _sum) = measure_memory_bandwidth(opts);

    benchmark_results.results.push(BenchmarkResult {
        size_mb,
        bandwidth_gb_s: bandwidth,
        simd_enabled: args.simd,
        parallel_enabled: args.parallel,
        affinity_enabled: args.affinity,
        iterations: 0,
        warmup_iterations: args.warmup,
        threads: args.threads.unwrap_or(1),
    });

    print_results(&benchmark_results, &args.format);
}
