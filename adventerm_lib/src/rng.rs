use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let state = if seed == 0 { 0x9E3779B97F4A7C15 } else { seed };
        Self { state }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    pub fn range(&mut self, low: usize, high_exclusive: usize) -> usize {
        debug_assert!(high_exclusive > low);
        let span = (high_exclusive - low) as u64;
        low + (self.next_u64() % span) as usize
    }

    pub fn chance(&mut self, num: u32, den: u32) -> bool {
        debug_assert!(den > 0);
        (self.next_u32() % den) < num
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_from_seed() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..32 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        let av: Vec<u64> = (0..16).map(|_| a.next_u64()).collect();
        let bv: Vec<u64> = (0..16).map(|_| b.next_u64()).collect();
        assert_ne!(av, bv);
    }

    #[test]
    fn range_within_bounds() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            let v = r.range(3, 10);
            assert!((3..10).contains(&v));
        }
    }
}
