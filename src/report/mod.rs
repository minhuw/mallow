use crate::system::cpu_info::CacheInfo;
use serde::Serialize;

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

#[derive(Serialize)]
pub struct BenchmarkConfig {
    pub simd_enabled: bool,
    pub parallel_enabled: bool,
    pub thread_count: usize,
    pub total_iterations: usize,
    pub warmup_iterations: usize,
    pub affinity_enabled: bool,
    pub core_ids: Vec<usize>,
    pub cpu_cache_info: CacheInfo,
}

pub fn print_results(results: &BenchmarkResults, format: &str) {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&results).unwrap());
        }
        "csv" => {
            println!("size_mb,bandwidth_gb_s,simd,parallel,affinity,threads");
            for result in &results.results {
                println!(
                    "{:.1},{:.2},{},{},{},{}",
                    result.size_mb,
                    result.bandwidth_gb_s,
                    result.simd_enabled,
                    result.parallel_enabled,
                    result.affinity_enabled,
                    result.threads
                );
            }
        }
        _ => {
            println!("\nMemory Read Bandwidth Benchmark");
            println!("================================");
            println!(
                "Running {} iterations ({} warmup) for each size",
                results.config.total_iterations, results.config.warmup_iterations
            );
            if results.config.parallel_enabled {
                println!(
                    "Parallel execution with {} threads",
                    results.config.thread_count
                );
                if results.config.affinity_enabled {
                    println!("Core affinity enabled: {:?}", results.config.core_ids);
                }
            }
            if results.config.simd_enabled {
                println!("SIMD enabled (16-wide u32)");
            }
            println!("\nBuffer Size\tBandwidth (GB/s)\tFlags\t\tThreads");
            println!("------------------------------------------------------------");
            for result in &results.results {
                let flags = format!(
                    "SIMD={}, PAR={}, AFF={}",
                    result.simd_enabled, result.parallel_enabled, result.affinity_enabled
                );
                println!(
                    "{:.1} MB\t{:.2} GB/s\t{}\t{}",
                    result.size_mb, result.bandwidth_gb_s, flags, result.threads
                );
            }
        }
    }
}
