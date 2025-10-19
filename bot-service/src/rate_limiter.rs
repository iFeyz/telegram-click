use std::time::Duration;

pub struct AdaptiveRateLimiter {
    min_interval: Duration,
    max_interval: Duration,
}

impl AdaptiveRateLimiter {
    pub fn new() -> Self {
        Self {
            min_interval: Duration::from_secs(1),
            max_interval: Duration::from_secs(30),
        }
    }

    pub fn calculate_interval(&self, active_users: usize) -> Duration {
        match active_users {
            0..=100 => Duration::from_secs(1),
            101..=500 => Duration::from_secs(3),
            501..=1000 => Duration::from_secs(5),
            1001..=3000 => Duration::from_secs(10),
            _ => Duration::from_secs(30),
        }
    }

    pub fn calculate_batch_size(&self, active_users: usize) -> usize {
        match active_users {
            0..=100 => 20,
            101..=500 => 30,
            501..=1000 => 25,
            _ => 20,
        }
    }
}

impl Default for AdaptiveRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_interval_low_users() {
        let limiter = AdaptiveRateLimiter::new();
        assert_eq!(limiter.calculate_interval(50), Duration::from_secs(1));
        assert_eq!(limiter.calculate_interval(100), Duration::from_secs(1));
    }

    #[test]
    fn test_calculate_interval_medium_users() {
        let limiter = AdaptiveRateLimiter::new();
        assert_eq!(limiter.calculate_interval(200), Duration::from_secs(3));
        assert_eq!(limiter.calculate_interval(500), Duration::from_secs(3));
    }

    #[test]
    fn test_calculate_interval_high_users() {
        let limiter = AdaptiveRateLimiter::new();
        assert_eq!(limiter.calculate_interval(750), Duration::from_secs(5));
        assert_eq!(limiter.calculate_interval(2000), Duration::from_secs(10));
        assert_eq!(limiter.calculate_interval(5000), Duration::from_secs(30));
    }

    #[test]
    fn test_calculate_batch_size() {
        let limiter = AdaptiveRateLimiter::new();
        assert_eq!(limiter.calculate_batch_size(50), 20);
        assert_eq!(limiter.calculate_batch_size(200), 30);
        assert_eq!(limiter.calculate_batch_size(750), 25);
        assert_eq!(limiter.calculate_batch_size(5000), 20);
    }
}
