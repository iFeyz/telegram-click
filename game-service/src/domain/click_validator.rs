use chrono::{DateTime, Duration, Utc};
use shared::{Result, ServiceError, UserId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};


pub struct ClickValidator {
    recent_clicks: Arc<RwLock<HashMap<UserId, Vec<DateTime<Utc>>>>>,
    max_clicks_per_second: u32,
}

impl ClickValidator {

    pub fn new(max_clicks_per_second: u32) -> Self {
        Self {
            recent_clicks: Arc::new(RwLock::new(HashMap::new())),
            max_clicks_per_second,
        }
    }


    pub fn validate_click(&self, user_id: &UserId, timestamp: DateTime<Utc>) -> Result<()> {
        let mut recent_clicks = self
            .recent_clicks
            .write()
            .map_err(|e| ServiceError::Internal(format!("Lock error: {}", e)))?;

        let user_clicks = recent_clicks.entry(*user_id).or_insert_with(Vec::new);

        let cutoff = timestamp - Duration::seconds(1);
        user_clicks.retain(|&click_time| click_time > cutoff);

        if user_clicks.len() >= self.max_clicks_per_second as usize {
            return Err(ServiceError::RateLimitExceeded);
        }

        user_clicks.push(timestamp);

        Ok(())
    }

    pub fn cleanup_old_data(&self) {
        if let Ok(mut recent_clicks) = self.recent_clicks.write() {
            let cutoff = Utc::now() - Duration::seconds(10);
            recent_clicks.retain(|_, clicks| {
                clicks.iter().any(|&click_time| click_time > cutoff)
            });
        }
    }

    pub fn get_current_rate(&self, user_id: &UserId) -> u32 {
        if let Ok(recent_clicks) = self.recent_clicks.read() {
            if let Some(clicks) = recent_clicks.get(user_id) {
                let cutoff = Utc::now() - Duration::seconds(1);
                return clicks.iter().filter(|&&t| t > cutoff).count() as u32;
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_click_validation_within_limit() {
        let validator = ClickValidator::new(10);
        let user_id = UserId::new();
        let now = Utc::now();

        for _ in 0..10 {
            assert!(validator.validate_click(&user_id, now).is_ok());
        }
    }

    #[test]
    fn test_click_validation_exceeds_limit() {
        let validator = ClickValidator::new(5);
        let user_id = UserId::new();
        let now = Utc::now();

        for _ in 0..5 {
            validator.validate_click(&user_id, now).unwrap();
        }

        assert!(validator.validate_click(&user_id, now).is_err());
    }

    #[test]
    fn test_cleanup_old_clicks() {
        let validator = ClickValidator::new(10);
        let user_id = UserId::new();
        let old_time = Utc::now() - Duration::seconds(2);

        for _ in 0..5 {
            validator.validate_click(&user_id, old_time).unwrap();
        }

        let now = Utc::now();
        assert!(validator.validate_click(&user_id, now).is_ok());
    }

    #[test]
    fn test_different_users_isolated() {
        let validator = ClickValidator::new(5);
        let user_id_1 = UserId::new();
        let user_id_2 = UserId::new();
        let now = Utc::now();

        for _ in 0..5 {
            validator.validate_click(&user_id_1, now).unwrap();
        }

        assert!(validator.validate_click(&user_id_2, now).is_ok());
    }

    #[test]
    fn test_get_current_rate() {
        let validator = ClickValidator::new(10);
        let user_id = UserId::new();
        let now = Utc::now();

        assert_eq!(validator.get_current_rate(&user_id), 0);

        for _ in 0..3 {
            validator.validate_click(&user_id, now).unwrap();
        }
        assert_eq!(validator.get_current_rate(&user_id), 3);
    }

    #[test]
    fn test_cleanup_old_data_removes_inactive_users() {
        let validator = ClickValidator::new(10);
        let user_id = UserId::new();
        let old_time = Utc::now() - Duration::seconds(15);

        for _ in 0..5 {
            validator.validate_click(&user_id, old_time).unwrap();
        }

        validator.cleanup_old_data();

        assert_eq!(validator.get_current_rate(&user_id), 0);
    }

    #[test]
    fn test_rate_limit_respects_window() {
        let validator = ClickValidator::new(3);
        let user_id = UserId::new();
        let now = Utc::now();

        for _ in 0..3 {
            validator.validate_click(&user_id, now).unwrap();
        }

        assert!(validator.validate_click(&user_id, now).is_err());

        let later = now + Duration::seconds(2);
        assert!(validator.validate_click(&user_id, later).is_ok());
    }

    #[test]
    fn test_concurrent_users_at_limit() {
        let validator = ClickValidator::new(10);
        let now = Utc::now();

        for _ in 0..10 {
            let user_id = UserId::new();
            assert!(validator.validate_click(&user_id, now).is_ok());
        }
    }

    #[test]
    fn test_rate_limit_error_type() {
        let validator = ClickValidator::new(1);
        let user_id = UserId::new();
        let now = Utc::now();

        validator.validate_click(&user_id, now).unwrap();

        let result = validator.validate_click(&user_id, now);
        assert!(matches!(result, Err(ServiceError::RateLimitExceeded)));
    }
}
