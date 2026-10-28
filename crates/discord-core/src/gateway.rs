//! Gateway v10 bot transport. No user tokens, compression, or privileged token logging.
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::mpsc,
    time::{interval_at, timeout, Instant, MissedTickBehavior},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub enum Event {
    Dispatch(String, Value),
    Status(String),
}
#[derive(Default)]
struct Session {
    sequence: Option<u64>,
    id: Option<String>,
    resume_url: Option<String>,
}
enum Reconnect {
    Retry,
    Fresh,
    Fatal,
}

// Dropping/aborting the owning task closes the socket and cancels all timers.
pub async fn run(token: Arc<str>, events: mpsc::Sender<Event>) {
    let mut session = Session::default();
    let mut backoff = 1u64;
    loop {
        if events
            .send(Event::Status("Connecting".into()))
            .await
            .is_err()
        {
            return;
        }
        let began = Instant::now();
        match connection(&token, &events, &mut session).await {
            Reconnect::Fatal => {
                let _ = events
                    .send(Event::Status(
                        "Gateway rejected credentials/intents; check bot configuration".into(),
                    ))
                    .await;
                return;
            }
            Reconnect::Fresh => session = Session::default(),
            Reconnect::Retry => {}
        }
        if events.is_closed() {
            return;
        }
        if began.elapsed() > Duration::from_secs(60) {
            backoff = 1;
        }
        let _ = events
            .send(Event::Status("Disconnected; reconnecting".into()))
            .await;
        let jitter = rand::thread_rng().gen_range(0..1000);
        tokio::time::sleep(Duration::from_secs(backoff) + Duration::from_millis(jitter)).await;
        backoff = (backoff * 2).min(60);
    }
}
async fn connection(token: &str, events: &mpsc::Sender<Event>, session: &mut Session) -> Reconnect {
    let base = session
        .resume_url
        .as_deref()
        .unwrap_or("wss://gateway.discord.gg");
    let url = format!("{base}/?v=10&encoding=json");
    connection_at(&url, token, events, session).await
}
async fn connection_at(
    url: &str,
    token: &str,
    events: &mpsc::Sender<Event>,
    session: &mut Session,
) -> Reconnect {
    let Ok(Ok((mut socket, _))) = timeout(Duration::from_secs(20), connect_async(url)).await else {
        return Reconnect::Retry;
    };
    let Ok(Some(Ok(Message::Text(hello)))) = timeout(Duration::from_secs(20), socket.next()).await
    else {
        return Reconnect::Retry;
    };
    let Ok(hello) = serde_json::from_str::<Value>(&hello) else {
        return Reconnect::Retry;
    };
    let Some(ms) = hello["d"]["heartbeat_interval"]
        .as_u64()
        .filter(|ms| *ms > 0 && *ms <= 300_000)
    else {
        return Reconnect::Retry;
    };
    if hello["op"] != 10 {
        return Reconnect::Retry;
    }
    let auth = if let (Some(id), Some(seq)) = (&session.id, session.sequence) {
        json!({"op":6,"d":{"token":token,"session_id":id,"seq":seq}})
    } else {
        json!({"op":2,"d":{"token":token,"intents":1|512|4096|32768,
            "properties":{"os":std::env::consts::OS,"browser":"rust-desktop","device":"rust-desktop"}}})
    };
    if socket
        .send(Message::Text(auth.to_string().into()))
        .await
        .is_err()
    {
        return Reconnect::Retry;
    }
    let jitter = rand::thread_rng().gen_range(0..ms);
    let mut heartbeat = interval_at(
        Instant::now() + Duration::from_millis(jitter),
        Duration::from_millis(ms),
    );
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut awaiting_ack = false;
    loop {
        tokio::select! {
            _ = events.closed() => return Reconnect::Fatal,
            _ = heartbeat.tick() => {
                if awaiting_ack { return Reconnect::Retry; }
                if socket.send(Message::Text(json!({"op":1,"d":session.sequence}).to_string().into())).await.is_err() { return Reconnect::Retry; }
                awaiting_ack = true;
            }
            frame = socket.next() => {
                match frame {
                    Some(Ok(Message::Text(text))) => {
                        let Ok(value) = serde_json::from_str::<Value>(&text) else { return Reconnect::Retry; };
                        if let Some(seq) = value["s"].as_u64() { session.sequence = Some(seq); }
                        match value["op"].as_u64() {
                            Some(0) => {
                                let name = value["t"].as_str().unwrap_or("").to_owned();
                                let data = value["d"].clone();
                                if name == "READY" {
                                    session.id = data["session_id"].as_str().map(str::to_owned);
                                    // Only accept Discord's secure resume host, never arbitrary token destinations.
                                    session.resume_url = data["resume_gateway_url"].as_str()
                                        .filter(|url| *url == "wss://gateway.discord.gg")
                                        .map(str::to_owned);
                                }
                                if events.send(Event::Dispatch(name, data)).await.is_err() { return Reconnect::Fatal; }
                            }
                            Some(1) => {
                                if socket.send(Message::Text(json!({"op":1,"d":session.sequence}).to_string().into())).await.is_err() { return Reconnect::Retry; }
                                awaiting_ack = true;
                            }
                            Some(7) => return Reconnect::Retry,
                            Some(9) => return if value["d"].as_bool() == Some(true) { Reconnect::Retry } else { Reconnect::Fresh },
                            Some(11) => awaiting_ack = false,
                            _ => {}
                        }
                    }
                    Some(Ok(Message::Ping(bytes))) => {
                        if socket.send(Message::Pong(bytes)).await.is_err() { return Reconnect::Retry; }
                    }
                    Some(Ok(Message::Close(close))) => {
                        return match close.map(|c| u16::from(c.code)) {
                            Some(4004 | 4010 | 4011 | 4012 | 4013 | 4014) => Reconnect::Fatal,
                            Some(4007 | 4009) => Reconnect::Fresh,
                            _ => Reconnect::Retry,
                        };
                    }
                    None | Some(Err(_)) => return Reconnect::Retry,
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;

    #[tokio::test]
    async fn identifies_dispatches_and_heartbeats_with_sequence() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            ws.send(Message::Text(
                json!({"op":10,"d":{"heartbeat_interval":100}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            let auth = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let auth: Value = serde_json::from_str(&auth).unwrap();
            assert_eq!(auth["op"], 2);
            assert_eq!(auth["d"]["token"], "test-only");
            ws.send(Message::Text(json!({"op":0,"s":42,"t":"READY","d":{"session_id":"session","resume_gateway_url":"wss://gateway.discord.gg"}}).to_string().into())).await.unwrap();
            loop {
                let beat = ws.next().await.unwrap().unwrap().into_text().unwrap();
                let beat: Value = serde_json::from_str(&beat).unwrap();
                assert_eq!(beat["op"], 1);
                ws.send(Message::Text(json!({"op":11,"d":null}).to_string().into()))
                    .await
                    .unwrap();
                if beat["d"] == 42 {
                    break;
                }
                // The jittered first heartbeat can legitimately precede READY.
                assert!(beat["d"].is_null());
            }
            ws.send(Message::Text(json!({"op":7,"d":null}).to_string().into()))
                .await
                .unwrap();
        });
        let (tx, mut rx) = mpsc::channel(8);
        let mut session = Session::default();
        let result = timeout(
            Duration::from_secs(3),
            connection_at(&url, "test-only", &tx, &mut session),
        )
        .await
        .unwrap();
        assert!(matches!(result, Reconnect::Retry));
        assert_eq!(session.sequence, Some(42));
        assert_eq!(session.id.as_deref(), Some("session"));
        assert!(matches!(rx.recv().await, Some(Event::Dispatch(name, _)) if name == "READY"));
        server.await.unwrap();
    }
    #[tokio::test]
    async fn resumes_and_reconnects_when_heartbeat_ack_is_missing() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            ws.send(Message::Text(
                json!({"op":10,"d":{"heartbeat_interval":20}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            let auth = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let auth: Value = serde_json::from_str(&auth).unwrap();
            assert_eq!(auth["op"], 6);
            assert_eq!(auth["d"]["seq"], 42);
            let beat = ws.next().await.unwrap().unwrap().into_text().unwrap();
            assert_eq!(serde_json::from_str::<Value>(&beat).unwrap()["op"], 1);
            // Keep the socket open but deliberately omit ACK.
            tokio::time::sleep(Duration::from_millis(150)).await;
        });
        let (tx, _rx) = mpsc::channel(8);
        let mut session = Session {
            sequence: Some(42),
            id: Some("session".into()),
            resume_url: None,
        };
        assert!(matches!(
            timeout(
                Duration::from_secs(3),
                connection_at(&url, "test-only", &tx, &mut session)
            )
            .await
            .unwrap(),
            Reconnect::Retry
        ));
        server.await.unwrap();
    }
}
