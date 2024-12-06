mod config;
mod services;
mod utils;
pub mod models;

use crate::config::Secrets;
use crate::services::{api, pid};
use crate::utils::storage::MatchStorage;
use std::io::Write;
use std::path::PathBuf;
use std::{io, process};
use std::sync::Arc;
use oauth2::{basic::BasicClient, reqwest::async_http_client, AuthUrl, ClientId, ClientSecret, TokenUrl};
use oauth2::{AuthorizationCode, CsrfToken, PkceCodeChallenge, RedirectUrl, RefreshToken, Scope, TokenResponse};
use tokio::sync::watch;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use crate::api::{upload_video, delete_video_from_folder};
use std::fs;
use serde_json::Value;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    // Génère ou récupère le token d'accès
    if let Err(e) = get_token().await {
        eprintln!("Erreur lors de la génération ou récupération du token OAuth2 : {}", e);
        process::exit(1);
    }

    let secrets = Secrets::from_env().expect("Failed to load secrets");
    let match_storage = Arc::new(Mutex::new(MatchStorage::new()));

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    if !pid::is_process_running("chrome.exe") {
        println!("chrome.exe non détecté. Le programme va se terminer.");
        process::exit(1);
    }

    println!("chrome.exe détecté. Lancement des tâches.");

    let mut handles = vec![];
    for puuid in &secrets.summoner_puuids {
        let puuid = puuid.clone();
        let match_storage = Arc::clone(&match_storage);
        let folder_path = secrets.folder_path.clone();
        let api_key = secrets.riot_api_key.clone();
        let shutdown_rx = shutdown_rx.clone();
        let friends_puuids = secrets.friend_puuids.clone();

        handles.push(tokio::spawn(async move {
            let mut is_first_loop = true; // Indique si c'est la première boucle
            while !*shutdown_rx.borrow() {
                if let Err(e) =
                    process_puuid(&puuid, &api_key, &folder_path, &friends_puuids, match_storage.clone(), is_first_loop).await
                {
                    eprintln!("Erreur pour le PUUID {}: {}", puuid, e);
                }
                is_first_loop = false; // Après la première itération, bascule à false
                sleep(Duration::from_secs(60)).await;
            }
            println!("Arrêt de la tâche pour le PUUID {}", puuid);
        }));
    }

    while pid::is_process_running("chrome.exe") {
        sleep(Duration::from_secs(5)).await;
    }

    println!("chrome.exe fermé. Arrêt des tâches.");
    let _ = shutdown_tx.send(true);

    for handle in handles {
        let _ = handle.await;
    }

    println!("Programme terminé.");
    process::exit(0);
}


async fn process_puuid(
    puuid: &str,
    api_key: &str,
    folder_path: &PathBuf,
    friends_puuids: &Vec<String>,
    match_storage: Arc<Mutex<MatchStorage>>,
    is_first_loop: bool,
) -> Result<(), String> {
    let latest_match = api::fetch_latest_match_id(api_key, puuid).await?;
    upload_video(folder_path).await?;
    let mut storage = match_storage.lock().await;

    if is_first_loop {
        // Lors de la première boucle, stocke simplement le match ID
        println!(
            "Première boucle : enregistrement du match ID pour le PUUID {}",
            puuid
        );
        storage.store_match_id(puuid.to_string(), latest_match.to_string());
        return Ok(());
    }

    // Vérifie si le match ID a changé
    if storage.is_match_id_known(puuid, &latest_match) {
        println!("Match déjà connu pour le PUUID {}", puuid);
        return Ok(());
    }

    // Si nouveau match ID, récupère les détails, upload et delete
    let match_details = api::fetch_match_details(api_key, &latest_match, puuid, friends_puuids).await?;
    println!("Nouveau match pour le PUUID {}: {:?}", puuid, match_details);

    upload_video(folder_path).await?;
    delete_video_from_folder("video_path").await?;

    // Met à jour le match ID dans le storage
    storage.store_match_id(puuid.to_string(), latest_match.to_string());

    Ok(())
}

async fn get_token() -> Result<String, String> {
    // Vérifie si un fichier de jetons existe déjà
    if let Ok(token_content) = fs::read_to_string("token.json") {
        let token_data: Value = serde_json::from_str(&token_content).map_err(|e| e.to_string())?;
        if let Some(access_token) = token_data["access_token"].as_str() {
            return Ok(access_token.to_string());
        }
    }

    // Si aucun jeton valide, demande une nouvelle autorisation
    println!("Génération d'un nouveau jeton OAuth2.");
    generate_tokens().await?;
    let token_content = fs::read_to_string("token.json").map_err(|e| e.to_string())?;
    Ok(token_content)
}

async fn generate_tokens() -> Result<(), String> {
    // Chargez le fichier client_secret.json
    let client_secret = fs::read_to_string("client_secret.json")
        .map_err(|e| format!("Erreur lors de la lecture de client_secret.json : {}", e))?;
    let client_info: Value = serde_json::from_str(&client_secret)
        .map_err(|e| format!("Erreur de parsing JSON : {}", e))?;

    // Configurez le client OAuth2
    let client_id = ClientId::new(
        client_info["installed"]["client_id"]
            .as_str()
            .ok_or("client_id manquant")?
            .to_string(),
    );
    let client_secret = ClientSecret::new(
        client_info["installed"]["client_secret"]
            .as_str()
            .ok_or("client_secret manquant")?
            .to_string(),
    );
    let auth_uri = AuthUrl::new(
        client_info["installed"]["auth_uri"]
            .as_str()
            .ok_or("auth_uri manquant")?
            .to_string(),
    )
    .unwrap();
    let token_uri = TokenUrl::new(
        client_info["installed"]["token_uri"]
            .as_str()
            .ok_or("token_uri manquant")?
            .to_string(),
    )
    .unwrap();
    let redirect_uri = RedirectUrl::new("urn:ietf:wg:oauth:2.0:oob".to_string()).unwrap();

    let client = BasicClient::new(client_id, Some(client_secret), auth_uri, Some(token_uri))
        .set_redirect_uri(redirect_uri);

    // Générer l'URL d'autorisation
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(pkce_challenge)
        .add_scope(Scope::new("https://www.googleapis.com/auth/youtube.upload".to_string()))
        .url();

    println!("Ouvrez ce lien dans votre navigateur et autorisez l'accès :");
    println!("{}", auth_url);

    // Recevoir le code d'autorisation de l'utilisateur
    print!("Entrez le code d'autorisation ici : ");
    io::stdout().flush().unwrap();

    let mut auth_code = String::new();
    io::stdin()
        .read_line(&mut auth_code)
        .map_err(|e| format!("Erreur de lecture de l'entrée utilisateur : {}", e))?;
    let auth_code = AuthorizationCode::new(auth_code.trim().to_string());

    // Échanger le code d'autorisation contre les jetons
    let token_result = client
        .exchange_code(auth_code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
        .map_err(|e| format!("Erreur lors de l'échange du code : {}", e))?;

    println!("Jeton d'accès : {}", token_result.access_token().secret());
    if let Some(refresh_token) = token_result.refresh_token() {
        println!("Jeton d'actualisation : {}", refresh_token.secret());
    } else {
        println!("Aucun jeton d'actualisation reçu !");
    }

    // Sauvegarder les jetons dans un fichier
    let token_json = serde_json::to_string_pretty(&token_result)
        .map_err(|e| format!("Erreur lors de la sérialisation des jetons : {}", e))?;
    fs::write("token.json", token_json)
        .map_err(|e| format!("Erreur lors de l'écriture dans tokens.json : {}", e))?;

    println!("Jetons sauvegardés dans tokens.json");
    Ok(())
}

fn save_token(token: &str) -> Result<(), String> {
    fs::write("token.json", token).map_err(|e| format!("Erreur d'écriture des jetons : {}", e))
}