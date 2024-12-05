use std::collections::HashMap;

#[derive(Debug)]
pub struct MatchStorage {
    storage: HashMap<String, String>,
}

impl MatchStorage {
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub fn is_match_id_known(&self, puuid: &str, match_id: &str) -> bool {
        self.storage.get(puuid).map_or(false, |id| id == match_id)
    }

    pub fn store_match_id(&mut self, puuid: String, match_id: String) {
        self.storage.insert(puuid, match_id);
    }
}
