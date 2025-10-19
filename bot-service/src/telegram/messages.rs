pub fn format_welcome_message(
    username: &str,
    user_clicks: i64,
    global_clicks: i64,
    user_rank: i32,
    leaderboard: &[(i32, String, i64)],
) -> String {
    let leaderboard_text = format_leaderboard(leaderboard);

    format!(
        "🏆 Bitcoin Clicker Dashboard\n\
        ━━━━━━━━━━━━━━━━━\n\
        👤 Player: {}\n\
        🎯 Your Clicks: {}\n\
        🌍 Global Clicks: {}\n\
        📈 Your Rank: #{}\n\n\
        📊 Top Clickers:\n\
        {}",
        username, user_clicks, global_clicks, user_rank, leaderboard_text
    )
}

fn format_leaderboard(entries: &[(i32, String, i64)]) -> String {
    if entries.is_empty() {
        return "No players yet!".to_string();
    }

    entries
        .iter()
        .take(20)
        .map(|(rank, username, clicks)| {
            let medal = match rank {
                1 => "🥇",
                2 => "🥈",
                3 => "🥉",
                _ => "  ",
            };
            format!("{} {}. {} - {} clicks", medal, rank, username, clicks)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_welcome_message() {
        let leaderboard = vec![
            (1, "Alice".to_string(), 1000),
            (2, "Bob".to_string(), 500),
            (3, "Charlie".to_string(), 250),
        ];

        let message = format_welcome_message("TestUser", 100, 1850, 4, &leaderboard);

        assert!(message.contains("TestUser"));
        assert!(message.contains("100"));
        assert!(message.contains("1850"));
        assert!(message.contains("#4"));
        assert!(message.contains("Alice"));
    }

    #[test]
    fn test_format_leaderboard_empty() {
        let result = format_leaderboard(&[]);
        assert_eq!(result, "No players yet!");
    }

    #[test]
    fn test_format_leaderboard_medals() {
        let entries = vec![
            (1, "First".to_string(), 100),
            (2, "Second".to_string(), 90),
            (3, "Third".to_string(), 80),
            (4, "Fourth".to_string(), 70),
        ];

        let result = format_leaderboard(&entries);

        assert!(result.contains("🥇"));
        assert!(result.contains("🥈"));
        assert!(result.contains("🥉"));
        assert!(result.contains("First"));
        assert!(result.contains("Fourth"));
    }
}
