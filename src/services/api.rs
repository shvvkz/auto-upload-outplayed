use serde_json::Value;
use std::{fs::DirEntry, path::PathBuf, str::FromStr};
use oauth2::{basic::BasicClient, reqwest::async_http_client, AuthUrl, ClientId, ClientSecret, TokenUrl};
use oauth2::{RefreshToken, TokenResponse};
use reqwest::Client;
use serde_json::json;
use tokio::fs;


use crate::get_token;
use crate::models::types::{MatchDetails, QueueId, Role};

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


pub async fn fetch_match_details(
    api_key: &str,
    match_id: &str,
    summoner_puuid: &str,
    friends_puuids: &Vec<String>,
) -> Result<MatchDetails, String> {
    let url = format!(
        "https://europe.api.riotgames.com/lol/match/v5/matches/{}?api_key={}",
        match_id, api_key
    );

    // Récupère le JSON brut
    let response: Value = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let queue_id = response["info"]["queueId"]
        .as_u64()
        .ok_or("Queue ID not found")?
        .to_string();

    let queue = QueueId::from_str(&queue_id)?;

    let participants = response["info"]["participants"]
        .as_array()
        .ok_or("Participants not found")?;

    // Trouve les informations du joueur correspondant à notre PUUID
    let participant = participants
        .iter()
        .find(|p| p["puuid"].as_str() == Some(summoner_puuid))
        .ok_or("Participant with specified PUUID not found")?;

    let champion_name = participant["championName"]
        .as_str()
        .ok_or("Champion name not found")?
        .to_string();

    let role_str = participant["teamPosition"]
        .as_str()
        .ok_or("Role not found")?;

    let role = Role::from_str(role_str)?;

    let kills = participant["kills"]
        .as_u64()
        .ok_or("Kills not found")? as u32;

    let deaths = participant["deaths"]
        .as_u64()
        .ok_or("Deaths not found")? as u32;

    let assists = participant["assists"]
        .as_u64()
        .ok_or("Assists not found")? as u32;

    let friends: Option<Vec<String>> = {
        let friends_list: Vec<String> = participants
            .iter()
            .filter_map(|p| {
                let puuid = p["puuid"].as_str()?;
                if friends_puuids.contains(&puuid.to_string()) {
                    Some(puuid.to_string())
                } else {
                    None
                }
            })
            .collect();
        if friends_list.is_empty() {
            None
        } else {
            Some(friends_list)
        }
    };

    // Crée une instance de MatchDetails
    Ok(MatchDetails {
        type_queue: queue,
        champions_name: champion_name,
        role,
        kills,
        deaths,
        assists,
        friends,
    })
}

pub async fn upload_video(folder_path: &PathBuf) -> Result<(), String> {
    let file = check_folder_and_print_file_path(folder_path).await?;
    let video_data = fs::read(&file).await.map_err(|e| format!("Erreur de lecture du fichier : {}", e))?;

    let client_secret = std::fs::read_to_string("client_secret.json").map_err(|e| format!("Erreur lors du chargement du fichier client_secret.json : {}", e))?;
    let client_info: Value = serde_json::from_str(&client_secret).map_err(|e| format!("Erreur de parsing JSON : {}", e))?;

    let client_id = ClientId::new(client_info["installed"]["client_id"].as_str().unwrap().to_string());
    let client_secret = ClientSecret::new(client_info["installed"]["client_secret"].as_str().unwrap().to_string());
    let token_url = TokenUrl::new(client_info["installed"]["token_uri"].as_str().unwrap().to_string()).unwrap();

    // Chargez le jeton existant ou actualisez-le
    let access_token = if let Ok(token) = get_token().await {
        token
    } else {
        refresh_access_token(&client_id, &client_secret, &token_url, "VOTRE_REFRESH_TOKEN").await?
    };

    let client = reqwest::Client::new();
    let metadata = json!({
        "snippet": {
            "title": "Test Video Title",
            "description": "Test Video Description",
            "tags": ["Rust", "YouTube", "API"],
            "categoryId": "20" // Gaming
        },
        "status": {
            "privacyStatus": "unlisted"
        }
    });

    let init_url = "https://www.googleapis.com/upload/youtube/v3/videos?uploadType=resumable&part=snippet,status";
    let init_response = client
        .post(init_url)
        .bearer_auth(&access_token)
        .header("Content-Type", "application/json")
        .json(&metadata)
        .send()
        .await
        .map_err(|e| format!("Erreur lors de l'initialisation de l'upload : {}", e))?;

    if !init_response.status().is_success() {
        return Err(format!(
            "Erreur d'initialisation : {}",
            init_response.text().await.unwrap_or_default()
        ));
    }

    let upload_url = init_response
        .headers()
        .get("Location")
        .ok_or("En-tête Location manquant dans la réponse")?
        .to_str()
        .map_err(|_| "En-tête Location non valide")?;

    // Étape 5 : Téléversez la vidéo
    let upload_response = client
        .put(upload_url)
        .bearer_auth(&access_token)
        .body(video_data)
        .send()
        .await
        .map_err(|e| format!("Erreur lors du téléversement de la vidéo : {}", e))?;

    if !upload_response.status().is_success() {
        return Err(format!(
            "Erreur de téléversement : {}",
            upload_response.text().await.unwrap_or_default()
        ));
    }

    println!("Vidéo téléversée avec succès !");
    Ok(())
}


// Fonction pour rafraîchir le jeton d'accès
async fn refresh_access_token(
    client_id: &ClientId,
    client_secret: &ClientSecret,
    token_url: &TokenUrl,
    refresh_token: &str,
) -> Result<String, String> {
    let client = BasicClient::new(
        client_id.clone(),
        Some(client_secret.clone()),
        AuthUrl::new("https://accounts.google.com/o/oauth2/auth".to_string()).unwrap(),
        Some(token_url.clone()),
    );

    let refresh_token = RefreshToken::new(refresh_token.to_string());
    let token = client
        .exchange_refresh_token(&refresh_token)
        .request_async(async_http_client)
        .await
        .map_err(|e| format!("Erreur lors du rafraîchissement du jeton : {}", e))?;

    Ok(token.access_token().secret().to_string())
}

pub async fn check_folder_and_print_file_path(folder_path: &PathBuf) -> Result<String, String> {
    if folder_path.is_dir() {
        let mut entries = std::fs::read_dir(folder_path)
            .map_err(|e| e.to_string())?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir());

        if let Some(folder) = entries.next() {
            let folder_path = folder.path();
            let mut file_entries = std::fs::read_dir(&folder_path)
                .map_err(|e| e.to_string())?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_file());

            if let Some(file) = file_entries.next() {
                if let Some(file_name) = file.file_name().to_str() {
                    if file_name.ends_with(".mp4") {
                        return Ok(file.path().to_string_lossy().to_string());
                    }
                }
            }
        }
    } else {
        return Err("Provided path is not a directory".to_string());
    }
    Err("No file found in the provided directory".to_string())
}

pub async fn delete_video_from_folder(video_path: &str) -> Result<(), String> {
    println!("Deleting video from {}", video_path);
    Ok(())
}

