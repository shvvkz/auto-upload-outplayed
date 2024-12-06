use std::env;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Secrets {
    pub folder_path: PathBuf,
    pub riot_api_key: String,
    pub summoner_puuids: Vec<String>,
    pub friend_puuids: Vec<String>,
}

impl Secrets {
    pub fn from_env() -> Result<Self, String> {
        dotenv::dotenv().ok();

        let folder_path = env::var("FOLDER_PATH")
            .map(PathBuf::from)
            .map_err(|_| "FOLDER_PATH non défini".to_string())?;
        println!("folder_path: {:?}", folder_path);
        let riot_api_key =
            env::var("RIOT_API_KEY").map_err(|_| "RIOT_API_KEY non défini".to_string())?;

        let summoners_counter: i32 = env::var("SUMMONERS_COUNT")
            .map_err(|_| "SUMMONERS_COUNT non défini".to_string())?
            .parse()
            .map_err(|_| "SUMMONERS_COUNT doit être un nombre entier".to_string())?;

        let mut summoner_puuids: Vec<String> = Vec::new();

        for i in 0..summoners_counter {
            let summoner_puuid = env::var(format!("SUMMONER_PUUID_{}", i))
                .map_err(|_| format!("SUMMONER_PUUID_{} non défini", i).to_string())?;
            summoner_puuids.push(summoner_puuid);
        }

        let friends_counter: i32 = env::var("FRIENDS_COUNT")
            .map_err(|_| "FRIENDS_COUNT non défini".to_string())?
            .parse()
            .map_err(|_| "FRIENDS_COUNT doit être un nombre entier".to_string())?;

        let mut friend_puuids: Vec<String> = Vec::new();

        for i in 0..friends_counter {
            let friend_puuid = env::var(format!("FRIEND_PUUID_{}", i))
                .map_err(|_| format!("FRIEND_PUUID_{} non défini", i).to_string())?;
            friend_puuids.push(friend_puuid);
        }

        Ok(Self {
            folder_path,
            riot_api_key,
            summoner_puuids,
            friend_puuids,
        })
    }
}
