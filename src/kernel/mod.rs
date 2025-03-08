use std::simd::u32x16;

#[derive(Clone, Debug)]
pub enum Kernel {
    // Strided access with scalar operations
    Scalar,
    // Strided access with SIMD operations
    Simd,
    // SIMD stride with multiple vectors per iteration
    SimdMulti(usize), // number of vectors per stride
}

fn scalar_read(slice: &[u32], stride: usize) -> u64 {
    let mut sum = 0u64;
    for i in (0..slice.len()).step_by(stride) {
        sum = sum.wrapping_add(slice[i] as u64);
    }
    sum
}

fn simd_read(slice: &[u32], stride: usize) -> u64 {
    let mut sum = u32x16::splat(0);
    for i in (0..slice.len()).step_by(stride * 16) {
        if i + 16 <= slice.len() {
            sum += u32x16::from_slice(&slice[i..i + 16]);
        }
    }
    sum.horizontal_sum() as u64
}

fn simd_read_multi(slice: &[u32], stride: usize, vectors_per_stride: usize) -> u64 {
    let mut sums = vec![u32x16::splat(0); vectors_per_stride];
    for base in (0..slice.len()).step_by(stride * vectors_per_stride * 16) {
        for (i, sum) in sums.iter_mut().enumerate() {
            let idx = base + i * stride;
            if idx + 16 <= slice.len() {
                *sum += u32x16::from_slice(&slice[idx..idx + 16]);
            }
        }
    }
    sums.iter().map(|v| v.horizontal_sum() as u64).sum()
}

impl Kernel {
    pub fn run(&self, slice: &[u32], stride: usize) -> u64 {
        match self {
            Kernel::Scalar => scalar_read(slice, stride),
            Kernel::Simd => simd_read(slice, stride),
            Kernel::SimdMulti(vectors_per_stride) => {
                simd_read_multi(slice, stride, *vectors_per_stride)
            }
        }
    }
}

// Add horizontal_sum for u32x16
trait SimdExt {
    fn horizontal_sum(self) -> u32;
}

impl SimdExt for u32x16 {
    fn horizontal_sum(self) -> u32 {
        let arr = self.to_array();
        arr.iter().sum()
    }
}
