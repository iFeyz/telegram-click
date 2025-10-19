use shared::{Result, ServiceError, User, UserId, Username};
use crate::repository::UserRepository;


pub struct UserService {
    user_repo: UserRepository,
}

impl UserService {

    pub fn new(user_repo: UserRepository) -> Self {
        Self { user_repo }
    }


    pub async fn register_user(&self, telegram_id: i64, username: &str) -> Result<User> {
        let validated_username = Username::new(username)?;

        if let Ok(_) = self.user_repo.get_by_telegram_id(telegram_id).await {
            return Err(ServiceError::UserAlreadyExists(telegram_id.to_string()));
        }

        let user = self.user_repo.create_user(telegram_id, validated_username.as_str()).await?;

        tracing::info!(
            telegram_id = telegram_id,
            user_id = %user.id,
            username = %user.username,
            "User registered successfully"
        );

        Ok(user)
    }


    pub async fn get_or_create_user(&self, telegram_id: i64, username: &str) -> Result<(User, bool)> {
        match self.user_repo.get_by_telegram_id(telegram_id).await {
            Ok(user) => Ok((user, false)),
            Err(ServiceError::UserNotFound(_)) => {
                let user = self.register_user(telegram_id, username).await?;
                Ok((user, true))
            }
            Err(e) => Err(e),
        }
    }


    pub async fn get_user(&self, telegram_id: i64) -> Result<User> {
        self.user_repo.get_by_telegram_id(telegram_id).await
    }

    pub async fn get_user_by_id(&self, user_id: &UserId) -> Result<User> {
        self.user_repo.get_by_id(user_id).await
    }

 
    pub async fn change_username(&self, user_id: &UserId, new_username: &str) -> Result<()> {
        let validated = Username::new(new_username)?;

        self.user_repo.update_username(user_id, &validated).await?;

        tracing::info!(
            user_id = %user_id,
            new_username = new_username,
            "Username changed successfully"
        );

        Ok(())
    }

    pub async fn get_total_users(&self) -> Result<i64> {
        self.user_repo.count_total_users().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_username_validation_rejects_invalid() {
        assert!(Username::new("ab").is_err(), "Too short");
        assert!(Username::new(&"a".repeat(21)).is_err(), "Too long");
        assert!(Username::new("user@name").is_err(), "Invalid char @");
        assert!(Username::new("user name").is_err(), "Invalid char space");
        assert!(Username::new("user!").is_err(), "Invalid char !");
        assert!(Username::new("").is_err(), "Empty");
    }

    #[test]
    fn test_username_validation_accepts_valid() {
        assert!(Username::new("abc").is_ok(), "Minimum length");
        assert!(Username::new(&"a".repeat(20)).is_ok(), "Maximum length");
        assert!(Username::new("user123").is_ok(), "Alphanumeric");
        assert!(Username::new("user_name").is_ok(), "With underscore");
        assert!(Username::new("user-name").is_ok(), "With hyphen");
        assert!(Username::new("User_Name-123").is_ok(), "Mixed");
    }

    #[test]
    fn test_error_types() {
        // Invalid username error
        let error = Username::new("ab").unwrap_err();
        assert!(matches!(error, ServiceError::InvalidUsername(_)));

        // User already exists error
        let telegram_id = 123456789_i64;
        let error = ServiceError::UserAlreadyExists(telegram_id.to_string());
        assert!(matches!(error, ServiceError::UserAlreadyExists(_)));

        // User not found error
        let user_id = UserId::new();
        let error = ServiceError::UserNotFound(user_id.to_string());
        assert!(matches!(error, ServiceError::UserNotFound(_)));
    }


}
