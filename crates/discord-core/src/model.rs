use serde::{Deserialize, Serialize};

// Discord snowflakes must remain strings: JavaScript numbers lose precision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guild {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "type", default)]
    pub kind: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub author: User,
    #[serde(default)]
    pub content: String,
    pub timestamp: String,
}
