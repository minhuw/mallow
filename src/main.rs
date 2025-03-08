#![feature(portable_simd)]
use std::simd::f32x4;

fn vector_add_simd(a: &[f32], b: &[f32]) -> Vec<f32> {
    assert_eq!(a.len(), b.len());
    let mut result = Vec::with_capacity(a.len());

    // Process 4 elements at a time using SIMD
    let chunks = a.len() / 4;
    for i in 0..chunks {
        let start = i * 4;
        let va = f32x4::from_slice(&a[start..start + 4]);
        let vb = f32x4::from_slice(&b[start..start + 4]);
        let vc = va + vb;
        result.extend_from_slice(&vc.to_array());
    }

    // Handle remaining elements
    for i in (chunks * 4)..a.len() {
        result.push(a[i] + b[i]);
    }

    result
}

fn main() {
    // Example usage
    let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let b = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];

    println!("Input vectors:");
    println!("a: {:?}", a);
    println!("b: {:?}", b);

    let result = vector_add_simd(&a, &b);
    println!("\nResult of SIMD vector addition:");
    println!("result: {:?}", result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_add_simd() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let expected = vec![2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

        let result = vector_add_simd(&a, &b);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_vector_add_simd_non_multiple_of_four() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![1.0, 1.0, 1.0, 1.0, 1.0];
        let expected = vec![2.0, 3.0, 4.0, 5.0, 6.0];

        let result = vector_add_simd(&a, &b);
        assert_eq!(result, expected);
    }
}
