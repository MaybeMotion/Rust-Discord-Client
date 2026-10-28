use crate::model::{Channel, Guild, Message};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};

const MAX_CHANNELS_WITH_MESSAGES: usize = 32;
const MESSAGES_PER_CHANNEL: usize = 200;

#[derive(Default, Serialize, Clone)]
pub struct Snapshot {
    pub guilds: BTreeMap<String, Guild>,
    pub channels: BTreeMap<String, Channel>,
    pub messages: BTreeMap<String, VecDeque<Message>>,
    pub status: String,
}
#[derive(Default)]
pub struct Store {
    pub snapshot: Snapshot,
    recent: VecDeque<String>,
}
impl Store {
    pub fn insert(&mut self, message: Message) {
        let id = message.channel_id.clone();
        self.recent.retain(|key| key != &id);
        self.recent.push_back(id.clone());
        while self.recent.len() > MAX_CHANNELS_WITH_MESSAGES {
            if let Some(old) = self.recent.pop_front() {
                self.snapshot.messages.remove(&old);
            }
        }
        let queue = self.snapshot.messages.entry(id).or_default();
        if let Some(existing) = queue.iter_mut().find(|m| m.id == message.id) {
            *existing = message;
        } else {
            queue.push_back(message);
        }
        while queue.len() > MESSAGES_PER_CHANNEL {
            queue.pop_front();
        }
    }
    pub fn apply(&mut self, event: &str, data: Value) {
        match event {
            "READY" => {
                self.snapshot = Snapshot {
                    status: "Connected".into(),
                    ..Default::default()
                };
                self.recent.clear();
            }
            "RESUMED" => self.snapshot.status = "Connected".into(),
            "GUILD_CREATE" | "GUILD_UPDATE" => {
                if let Ok(guild) = serde_json::from_value::<Guild>(data.clone()) {
                    self.snapshot.guilds.insert(guild.id.clone(), guild.clone());
                    if let Some(channels) = data["channels"].as_array() {
                        for value in channels {
                            if let Ok(mut channel) =
                                serde_json::from_value::<Channel>(value.clone())
                            {
                                channel.guild_id = Some(guild.id.clone());
                                self.snapshot.channels.insert(channel.id.clone(), channel);
                            }
                        }
                    }
                }
            }
            "CHANNEL_CREATE" | "CHANNEL_UPDATE" => {
                if let Ok(channel) = serde_json::from_value::<Channel>(data) {
                    self.snapshot.channels.insert(channel.id.clone(), channel);
                }
            }
            "CHANNEL_DELETE" => {
                if let Some(id) = data["id"].as_str() {
                    self.snapshot.channels.remove(id);
                    self.snapshot.messages.remove(id);
                    self.recent.retain(|key| key != id);
                }
            }
            "GUILD_DELETE" => {
                if data["unavailable"].as_bool() == Some(true) {
                    return;
                }
                if let Some(id) = data["id"].as_str() {
                    self.snapshot.guilds.remove(id);
                    let removed: Vec<_> = self
                        .snapshot
                        .channels
                        .values()
                        .filter(|c| c.guild_id.as_deref() == Some(id))
                        .map(|c| c.id.clone())
                        .collect();
                    for key in removed {
                        self.snapshot.channels.remove(&key);
                        self.snapshot.messages.remove(&key);
                        self.recent.retain(|item| item != &key);
                    }
                }
            }
            "MESSAGE_CREATE" => {
                if let Ok(message) = serde_json::from_value(data) {
                    self.insert(message);
                }
            }
            "MESSAGE_UPDATE" => {
                if let (Some(channel), Some(id)) =
                    (data["channel_id"].as_str(), data["id"].as_str())
                {
                    if let Some(messages) = self.snapshot.messages.get_mut(channel) {
                        if let Some(message) = messages.iter_mut().find(|m| m.id == id) {
                            if let Some(content) = data["content"].as_str() {
                                message.content = content.into();
                            }
                        }
                    }
                }
            }
            "MESSAGE_DELETE" => {
                if let (Some(channel), Some(id)) =
                    (data["channel_id"].as_str(), data["id"].as_str())
                {
                    if let Some(messages) = self.snapshot.messages.get_mut(channel) {
                        messages.retain(|m| m.id != id);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::User;
    fn message(id: usize, channel: usize) -> Message {
        Message {
            id: id.to_string(),
            channel_id: channel.to_string(),
            author: User {
                id: "1".into(),
                username: "tester".into(),
            },
            content: "hello".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
        }
    }
    #[test]
    fn bounds_and_deduplicates_messages() {
        let mut store = Store::default();
        for id in 0..250 {
            store.insert(message(id, 1));
        }
        store.insert(message(249, 1));
        assert_eq!(store.snapshot.messages["1"].len(), 200);
        assert_eq!(store.snapshot.messages["1"].front().unwrap().id, "50");
        for channel in 2..40 {
            store.insert(message(1, channel));
        }
        assert_eq!(store.snapshot.messages.len(), 32);
        assert!(!store.snapshot.messages.contains_key("1"));
    }
    #[test]
    fn applies_partial_edits_and_deletion() {
        let mut store = Store::default();
        store.insert(message(1, 1));
        store.apply(
            "MESSAGE_UPDATE",
            serde_json::json!({"id":"1", "channel_id":"1", "content":"edited"}),
        );
        assert_eq!(store.snapshot.messages["1"][0].content, "edited");
        store.apply(
            "MESSAGE_DELETE",
            serde_json::json!({"id":"1", "channel_id":"1"}),
        );
        assert!(store.snapshot.messages["1"].is_empty());
    }
}
