use rand::Rng;
use serde::Serialize;
use std::simd::{u32x8, usizex8};

#[derive(Clone, Debug, Serialize)]
pub enum Kernel {
    // Strided access with scalar operations
    ScalarRead,
    ScalarWrite,
    // Strided access with SIMD operations
    SimdRead,
    SimdWrite,
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
    let offset = rand::rng().random_range(0..stride);
    // Create indices for gather: [0*stride, 1*stride, 2*stride, ..., 15*stride]
    let indices = usizex8::from_array(std::array::from_fn(|i| i * stride + offset));

    // Process strided elements in chunks
    let mut base = 0;
    while base + (7 * stride) < slice.len() {
        // Gather values from strided locations
        sum += u32x8::gather_or_default(&slice[base..], indices).horizontal_sum() as u64;

        base += stride * 8;
    }

    sum
}

pub fn scalar_write(slice: &mut [u32], stride: usize) -> u64 {
    let mut sum = 0u64;
    let len = slice.len();

    // Process 4 elements per iteration
    let unroll = 4;
    let main_iterations = len / (stride * unroll);
    let mut i = 0;

    // Main loop with 4x unrolling
    for _ in 0..main_iterations {
        // Write and accumulate values
        for j in 0..unroll {
            let idx = i + stride * j;
            let val = (idx as u32).wrapping_mul(7); // Some deterministic value
            slice[idx] = val;
            sum = sum.wrapping_add(val as u64);
        }
        i += stride * unroll;
    }

    // Handle remaining elements
    while i < len {
        let val = (i as u32).wrapping_mul(7);
        slice[i] = val;
        sum = sum.wrapping_add(val as u64);
        i += stride;
    }

    sum
}

pub fn simd_write(slice: &mut [u32], stride: usize) -> u64 {
    let mut sum: u64 = 0;
    let offset = rand::rng().random_range(0..stride);

    // Create indices for scatter: [0*stride, 1*stride, 2*stride, ..., 7*stride]
    let indices = usizex8::from_array(std::array::from_fn(|i| i * stride + offset));

    // Create values to write: [i*7, (i+1)*7, ..., (i+7)*7]
    let mut base = 0;
    while base + (7 * stride) < slice.len() {
        let values = u32x8::from_array(std::array::from_fn(|i| {
            ((base + i * stride) as u32).wrapping_mul(7)
        }));

        // Scatter values to strided locations
        unsafe {
            values.scatter_unchecked(&mut slice[base..], indices);
        }

        sum += values.horizontal_sum() as u64;
        base += stride * 8;
    }

    sum
}

impl Kernel {
    pub fn run(&self, slice: &mut [u32], stride: usize) -> u64 {
        match self {
            Kernel::ScalarRead => scalar_read(slice, stride),
            Kernel::ScalarWrite => scalar_write(slice, stride),
            Kernel::SimdRead => simd_read(slice, stride),
            Kernel::SimdWrite => simd_write(slice, stride),
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

// Add scatter support for u32x8
trait SimdScatter {
    unsafe fn scatter_unchecked(self, slice: &mut [u32], indices: usizex8);
}

impl SimdScatter for u32x8 {
    unsafe fn scatter_unchecked(self, slice: &mut [u32], indices: usizex8) {
        let values = self.to_array();
        let idx = indices.to_array();
        for i in 0..8 {
            *slice.get_unchecked_mut(idx[i]) = values[i];
        }
    }
}
