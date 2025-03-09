use serde::Serialize;
use std::simd::{u32x8, usizex8};

#[derive(Clone, Debug, Serialize)]
pub enum Kernel {
    // Strided access with scalar operations
    Scalar,
    // Strided access with SIMD operations
    Simd,
}

pub fn scalar_read(slice: &[u32], stride: usize) -> u64 {
    let mut sum = 0u64;
    let len = slice.len();

    // Process 4 elements per iteration
    let unroll = 4;
    let main_iterations = len / (stride * unroll);
    let mut i = 0;

    // Main loop with 4x unrolling
    for _ in 0..main_iterations {
        sum = sum
            .wrapping_add(slice[i] as u64)
            .wrapping_add(slice[i + stride] as u64)
            .wrapping_add(slice[i + stride * 2] as u64)
            .wrapping_add(slice[i + stride * 3] as u64);
        i += stride * unroll;
    }

    // Handle remaining elements
    while i < len {
        sum = sum.wrapping_add(slice[i] as u64);
        i += stride;
    }

    sum
}

pub fn simd_read(slice: &[u32], stride: usize) -> u64 {
    let mut sum: u64 = 0;

    // Create indices for gather: [0*stride, 1*stride, 2*stride, ..., 15*stride]
    let indices = usizex8::from_array(std::array::from_fn(|i| i * stride));

    // Process strided elements in chunks
    let mut base = 0;
    while base + (7 * stride) < slice.len() {
        // Gather values from strided locations
        sum += u32x8::gather_or_default(&slice[base..], indices).horizontal_sum() as u64;

        base += stride * 8;
    }

    sum
}

impl Kernel {
    pub fn run(&self, slice: &[u32], stride: usize) -> u64 {
        match self {
            Kernel::Scalar => scalar_read(slice, stride),
            Kernel::Simd => simd_read(slice, stride),
        }
    }
}

// Add horizontal_sum for u32x16
trait SimdExt {
    fn horizontal_sum(self) -> u32;
}

impl SimdExt for u32x8 {
    fn horizontal_sum(self) -> u32 {
        let arr = self.to_array();
        arr.iter().sum()
    }
}
