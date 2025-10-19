#[derive(Clone, Default)]
pub enum State {
    #[default]
    Idle,
    WaitingForNameChange { user_id: String },
}
