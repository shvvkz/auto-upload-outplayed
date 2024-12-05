mod config;
mod services;
mod utils;

use crate::config::Secrets;
use crate::services::{api, pid};
use crate::utils::storage::MatchStorage;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tokio::time::{sleep, Duration};
use std::process;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let secrets = Secrets::from_env().expect("Failed to load secrets");
    let match_storage = Arc::new(Mutex::new(MatchStorage::new()));

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    if !pid::is_process_running("Discord") {
        println!("Discord non détecté. Le programme va se terminer.");
        process::exit(1);
    }

    println!("Discord détecté. Lancement des tâches.");

    let mut handles = vec![];
    for puuid in &secrets.summoner_puuids {
        let puuid = puuid.clone();
        let match_storage = Arc::clone(&match_storage);
        let api_key = secrets.riot_api_key.clone();
        let shutdown_rx = shutdown_rx.clone();

        handles.push(tokio::spawn(async move {
            while !*shutdown_rx.borrow() {
                if let Err(e) = process_puuid(&puuid, &api_key, match_storage.clone()).await {
                    eprintln!("Erreur pour le PUUID {}: {}", puuid, e);
                }
                sleep(Duration::from_secs(60)).await;
            }
            println!("Arrêt de la tâche pour le PUUID {}", puuid);
        }));
    }

    while pid::is_process_running("Discord") {
        sleep(Duration::from_secs(5)).await;
    }

    println!("Discord fermé. Arrêt des tâches.");
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
    match_storage: Arc<Mutex<MatchStorage>>,
) -> Result<(), String> {
    let latest_match = api::fetch_latest_match_id(api_key, puuid).await?;

    let mut storage = match_storage.lock().unwrap();
    if storage.is_match_id_known(puuid, &latest_match) {
        return Ok(());
    }

    println!("Nouveau match pour le PUUID {}: {}", puuid, latest_match);
    storage.store_match_id(puuid.to_string(), latest_match);

    Ok(())
}
