use reqwest::blocking::Client;
use serde_json::Value;

pub async fn fetch_latest_match_id(api_key: &str, puuid: &str) -> Result<String, String> {
    let url = format!(
        "https://europe.api.riotgames.com/lol/match/v5/matches/by-puuid/{}/ids?start=0&count=1&api_key={}",
        puuid, api_key
    );

    let response: Vec<String> = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    println!("Response: {:?}", response);
    response
        .get(0)
        .cloned()
        .ok_or_else(|| "No match ID found".to_string())
}


pub fn fetch_match_details(api_key: &str, match_id: &str) -> Result<Value, String> {
    let url = format!(
        "https://europe.api.riotgames.com/lol/match/v5/matches/{}?api_key={}",
        match_id, api_key
    );

    Client::new()
        .get(&url)
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())
}

pub fn upload_video(video_path: &str) -> Result<(), String> {
    println!("Uploading video from {}", video_path);
    Ok(())
}

pub fn delete_video_from_folder(video_path: &str) -> Result<(), String> {
    println!("Deleting video from {}", video_path);
    Ok(())
}
