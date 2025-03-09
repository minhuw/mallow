use crate::kernel::Kernel;
use crate::system::cpu_info::CacheInfo;
use core_affinity::CoreId;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct BenchmarkConfig {
    pub size: usize,
    pub stride: usize,
    pub duration_secs: f64,
    pub warmup_iterations: usize,
    pub kernel: Kernel,
    pub thread_count: usize,
    #[serde(skip)]
    pub core_ids: Vec<CoreId>,
    pub cpu_cache_info: CacheInfo,
}

#[derive(Serialize)]
pub struct BenchmarkResult {
    pub size_mb: f64,
    pub bandwidth_gb_s: f64,
    pub simd_enabled: bool,
    pub parallel_enabled: bool,
    pub affinity_enabled: bool,
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub threads: usize,
}

#[derive(Serialize)]
pub struct BenchmarkResults {
    pub results: Vec<BenchmarkResult>,
    pub config: BenchmarkConfig,
}

pub fn print_results(results: &BenchmarkResults, format: &str) {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&results).unwrap());
        }
        "csv" => {
            println!("size_mb,bandwidth_gb_s,simd,parallel,affinity,threads,iterations");
            for result in &results.results {
                println!(
                    "{:.1},{:.2},{},{},{},{},{}",
                    result.size_mb,
                    result.bandwidth_gb_s,
                    result.simd_enabled,
                    result.parallel_enabled,
                    result.affinity_enabled,
                    result.threads,
                    result.iterations
                );
            }
        }
        _ => {
            println!("\nMemory Read Bandwidth Benchmark");
            println!("================================");
            println!(
                "Running for {:.1} seconds ({} warmup iterations)",
                results.config.duration_secs, results.config.warmup_iterations
            );
            let is_parallel = results.config.thread_count > 1;
            if is_parallel {
                println!(
                    "Parallel execution with {} threads",
                    results.config.thread_count
                );
                if !results.config.core_ids.is_empty() {
                    println!(
                        "Core affinity enabled: {:?}",
                        results
                            .config
                            .core_ids
                            .iter()
                            .map(|id| id.id)
                            .collect::<Vec<_>>()
                    );
                }
            }
            match results.config.kernel {
                Kernel::Simd => println!("SIMD enabled (8-wide u32)"),
                Kernel::Scalar => println!("Scalar operations"),
            }
            println!("\nBuffer Size\tBandwidth (GB/s)\tFlags\t\tThreads\tIterations");
            println!("------------------------------------------------------------------------");
            for result in &results.results {
                let flags = format!(
                    "SIMD={}, PAR={}, AFF={}",
                    result.simd_enabled, result.parallel_enabled, result.affinity_enabled
                );
                println!(
                    "{:.1} MB\t{:.2} GB/s\t{}\t{}\t{}",
                    result.size_mb, result.bandwidth_gb_s, flags, result.threads, result.iterations
                );
            }
        }
    }
}
