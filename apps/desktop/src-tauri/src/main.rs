#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use discord_core::{
    gateway::{self, Event},
    model::Message,
    rest::Rest,
    state::{Snapshot, Store},
};
use std::sync::Arc;
use tauri::{Emitter, Manager, State};
use tokio::sync::{mpsc, RwLock};

struct Backend {
    store: Arc<RwLock<Store>>,
    rest: Option<Rest>,
}
#[tauri::command]
async fn snapshot(backend: State<'_, Backend>) -> Result<Snapshot, String> {
    Ok(backend.store.read().await.snapshot.clone())
}
#[tauri::command]
async fn history(channel: String, backend: State<'_, Backend>) -> Result<Vec<Message>, String> {
    if let Some(rest) = &backend.rest {
        let mut messages = rest.history(&channel).await.map_err(|e| e.to_string())?;
        messages.reverse(); // API returns newest first.
        let mut store = backend.store.write().await;
        for message in &messages {
            store.insert(message.clone());
        }
        Ok(messages)
    } else {
        Ok(backend
            .store
            .read()
            .await
            .snapshot
            .messages
            .get(&channel)
            .map(|q| q.iter().cloned().collect())
            .unwrap_or_default())
    }
}
#[tauri::command]
async fn send_message(
    channel: String,
    content: String,
    backend: State<'_, Backend>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let rest = backend
        .rest
        .as_ref()
        .ok_or("Demo is read-only. Configure a bot token in the backend to send.")?;
    let message = rest
        .send(&channel, &content)
        .await
        .map_err(|e| e.to_string())?;
    backend.store.write().await.insert(message);
    app.emit("state-changed", ()).map_err(|e| e.to_string())
}
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![snapshot, history, send_message])
        .setup(|app| {
            let token = std::env::var("DISCORD_BOT_TOKEN").ok().filter(|v| !v.trim().is_empty());
            let mut store = Store::default();
            if token.is_none() {
                store.apply("GUILD_CREATE", serde_json::json!({"id":"1","name":"Demo workspace","channels":[{"id":"2","name":"welcome","type":0}]}));
                store.apply("MESSAGE_CREATE", serde_json::json!({"id":"3","channel_id":"2","author":{"id":"4","username":"Rust starter"},"content":"The native shell is ready. Choose a theme or import your own JSON. Connect a bot to load its accessible guilds.","timestamp":"2026-01-01T00:00:00Z"}));
                store.snapshot.status = "Demo · offline".into();
            }
            let rest = token.as_deref().map(Rest::new).transpose()?;
            let store = Arc::new(RwLock::new(store));
            app.manage(Backend { store: store.clone(), rest });
            if let Some(token) = token {
                let (tx, mut rx) = mpsc::channel(256);
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(gateway::run(Arc::from(token), tx));
                tauri::async_runtime::spawn(async move {
                    while let Some(event) = rx.recv().await {
                        {
                            let mut state = store.write().await;
                            match event {
                                Event::Dispatch(name, data) => state.apply(&name, data),
                                Event::Status(status) => state.snapshot.status = status,
                            }
                            // Batch bursts so the UI sees at most ~20 invalidations/second.
                            while let Ok(event) = rx.try_recv() {
                                match event {
                                    Event::Dispatch(name, data) => state.apply(&name, data),
                                    Event::Status(status) => state.snapshot.status = status,
                                }
                            }
                        }
                        let _ = handle.emit("state-changed", ());
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("desktop runtime failed");
}
