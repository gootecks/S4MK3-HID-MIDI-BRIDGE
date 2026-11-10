//! Transfer-time statistics for the stream tool's summary lines.

use std::time::Duration;

use serde::Serialize;

/// Transfer times in milliseconds; `p95` is the nearest-rank 95th percentile.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Millis {
    pub min: f64,
    pub median: f64,
    pub p95: f64,
    pub max: f64,
}

/// `None` for an empty list.
#[must_use]
pub fn summarize(times: &[Duration]) -> Option<Millis> {
    let mut sorted = times.to_vec();
    sorted.sort_unstable();
    let count = sorted.len();
    let (first, last) = (sorted.first()?, sorted.last()?);
    let median = if count % 2 == 1 {
        sorted[count / 2]
    } else {
        (sorted[count / 2 - 1] + sorted[count / 2]) / 2
    };
    let p95 = sorted[(count * 95).div_ceil(100) - 1];
    let ms = |time: Duration| time.as_secs_f64() * 1000.0;
    Some(Millis {
        min: ms(*first),
        median: ms(median),
        p95: ms(p95),
        max: ms(*last),
    })
}

/// `wanted` frame numbers out of `count`, evenly spaced from the first to the
/// last (both included). Every number when `count <= wanted`.
#[must_use]
pub fn sample_positions(count: usize, wanted: usize) -> Vec<usize> {
    if count <= wanted {
        return (0..count).collect();
    }
    let steps = wanted - 1;
    (0..wanted)
        .map(|index| (index * (count - 1) + steps / 2) / steps)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn millis(values: &[u64]) -> Vec<Duration> {
        values.iter().map(|ms| Duration::from_millis(*ms)).collect()
    }

    #[test]
    fn empty_list_has_no_statistics() {
        assert_eq!(summarize(&[]), None);
    }

    #[test]
    fn odd_and_even_counts_use_the_middle_and_nearest_rank() {
        let odd = summarize(&millis(&[9, 1, 5])).unwrap_or_else(|| unreachable!());
        assert!((odd.min - 1.0).abs() < 1e-9);
        assert!((odd.median - 5.0).abs() < 1e-9);
        assert!((odd.p95 - 9.0).abs() < 1e-9);

        let hundred: Vec<u64> = (1..=100).collect();
        let stats = summarize(&millis(&hundred)).unwrap_or_else(|| unreachable!());
        assert!((stats.median - 50.5).abs() < 1e-9);
        assert!((stats.p95 - 95.0).abs() < 1e-9);
        assert!((stats.max - 100.0).abs() < 1e-9);
    }

    #[test]
    fn samples_run_from_the_first_to_the_last_frame() {
        assert_eq!(sample_positions(0, 5), Vec::<usize>::new());
        assert_eq!(sample_positions(3, 5), vec![0, 1, 2]);
        assert_eq!(sample_positions(120, 5), vec![0, 30, 60, 89, 119]);
        assert_eq!(sample_positions(1800, 5), vec![0, 450, 900, 1349, 1799]);
    }
}
