use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, WebAppInfo};

pub fn make_game_keyboard(mini_app_url: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::web_app(
                "🎮 PLAY GAME",
                WebAppInfo {
                    url: mini_app_url.parse().expect("Invalid Mini App URL"),
                },
            ),
        ],
        vec![
            InlineKeyboardButton::callback("👤 Change Name", "change_name"),
            InlineKeyboardButton::callback("🔄 Refresh", "refresh"),
        ],
    ])
}

pub fn make_username_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("🎲 Random", "username_random"),
        InlineKeyboardButton::callback("✍️ Custom", "username_custom"),
    ]])
}
