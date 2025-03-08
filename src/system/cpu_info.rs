use raw_cpuid::CpuId;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct CacheInfo {
    pub l1d_line_size: Option<usize>,
    pub l1d_size_kb: Option<usize>,
    pub l1d_sets: Option<usize>,
    pub l1d_associativity: Option<usize>,
    pub l2_line_size: Option<usize>,
    pub l2_size_kb: Option<usize>,
    pub l2_sets: Option<usize>,
    pub l2_associativity: Option<usize>,
    pub l3_line_size: Option<usize>,
    pub l3_size_kb: Option<usize>,
    pub l3_sets: Option<usize>,
    pub l3_associativity: Option<usize>,
}

pub fn get_cpu_info() -> CacheInfo {
    let cpuid = CpuId::new();
    let mut info = CacheInfo {
        l1d_line_size: None,
        l1d_size_kb: None,
        l1d_sets: None,
        l1d_associativity: None,
        l2_line_size: None,
        l2_size_kb: None,
        l2_sets: None,
        l2_associativity: None,
        l3_line_size: None,
        l3_size_kb: None,
        l3_sets: None,
        l3_associativity: None,
    };

    if let Some(cparams) = cpuid.get_cache_parameters() {
        for cache in cparams {
            match cache.level() {
                1 if cache.cache_type() == raw_cpuid::CacheType::Data => {
                    info.l1d_line_size = Some(cache.coherency_line_size());
                    info.l1d_sets = Some(cache.sets());
                    info.l1d_associativity = Some(cache.associativity());
                    info.l1d_size_kb = Some(
                        cache.sets() * cache.associativity() * cache.coherency_line_size() / 1024,
                    );
                }
                2 => {
                    info.l2_line_size = Some(cache.coherency_line_size());
                    info.l2_sets = Some(cache.sets());
                    info.l2_associativity = Some(cache.associativity());
                    info.l2_size_kb = Some(
                        cache.sets() * cache.associativity() * cache.coherency_line_size() / 1024,
                    );
                }
                3 => {
                    info.l3_line_size = Some(cache.coherency_line_size());
                    info.l3_sets = Some(cache.sets());
                    info.l3_associativity = Some(cache.associativity());
                    info.l3_size_kb = Some(
                        cache.sets() * cache.associativity() * cache.coherency_line_size() / 1024,
                    );
                }
                _ => {}
            }
        }
    }

    info
}
