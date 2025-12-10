use serde::{Deserialize, Serialize};

// Slack Event API のリクエスト型
#[derive(Deserialize, Debug)]
pub struct SlackEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub challenge: Option<String>,
    pub event: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct ReactionAddedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub user: String,
    pub item: EventItem,
    pub reaction: String,
}

#[derive(Deserialize, Debug)]
pub struct EventItem {
    pub channel: String,
    pub ts: String,
}

#[derive(Deserialize, Debug)]
pub struct EmojiChangedEvent {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub event_type: String,
    pub subtype: String,
    pub name: String,
}

// Slack API レスポンス型
#[derive(Deserialize)]
pub struct MessageHistory {
    pub ok: bool,
    pub messages: Option<Vec<Message>>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct Message {
    pub user: Option<String>,
}

#[derive(Deserialize)]
pub struct UserInfoResponse {
    pub user: Option<User>,
}

#[derive(Deserialize)]
pub struct User {
    pub name: String,
    #[serde(rename = "real_name")]
    pub real_name: Option<String>,
}

// Slack API リクエスト型
#[derive(Serialize)]
pub struct SlackMessage {
    pub channel: String,
    pub text: String,
}
