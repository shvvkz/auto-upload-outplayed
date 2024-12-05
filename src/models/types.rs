#[derive(Debug)]
pub struct MatchDetails {
    pub match_id: String,
    pub game_duration: u64,
    pub participants: Vec<String>,
}
