use worker::*;
use crate::types::*;

/// Checks if a message is a threaded reply (not the parent message)
pub fn is_threaded_reply(thread_ts: Option<&str>, message_ts: &str) -> bool {
    thread_ts.map_or(false, |ts| ts != message_ts)
}

pub async fn get_user_info(token: &str, user_id: &str) -> Result<String> {
    let url = format!("https://slack.com/api/users.info?user={}", user_id);

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", token))?;

    let request = Request::new_with_init(
        &url,
        RequestInit::new()
            .with_method(Method::Get)
            .with_headers(headers),
    )?;

    let mut response = Fetch::Request(request).send().await?;
    let user_info: UserInfoResponse = response.json().await?;

    let user_name = user_info.user
        .map(|user| user.real_name.unwrap_or(user.name))
        .unwrap_or_else(|| user_id.to_string());

    console_log!("✅ get_user_info completed: {} -> {}", user_id, user_name);
    Ok(user_name)
}

pub async fn get_team_info(token: &str) -> Result<String> {
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", token))?;

    let request = Request::new_with_init(
        "https://slack.com/api/team.info",
        RequestInit::new()
            .with_method(Method::Get)
            .with_headers(headers),
    )?;

    let mut response = Fetch::Request(request).send().await?;
    let response_text = response.text().await?;

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response_text) {
        if parsed.get("ok").and_then(|ok| ok.as_bool()) == Some(false) {
            let error_msg = parsed.get("error").and_then(|e| e.as_str()).unwrap_or("unknown error");
            console_log!("Team info API failed: {}", error_msg);
        }

        if let Some(domain) = parsed.get("team")
            .and_then(|team| team.get("domain"))
            .and_then(|domain| domain.as_str()) {
            console_log!("✅ get_team_info completed: domain = {}", domain);
            return Ok(domain.to_string());
        }
    }

    console_log!("Failed to get workspace domain, using fallback");
    Ok("yourworkspace".to_string())
}

pub async fn is_external_channel(token: &str, channel_id: &str) -> Result<bool> {
    let url = format!("https://slack.com/api/conversations.info?channel={}", channel_id);

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", token))?;

    let request = Request::new_with_init(
        &url,
        RequestInit::new()
            .with_method(Method::Get)
            .with_headers(headers),
    )?;

    let mut response = Fetch::Request(request).send().await?;
    let response_text = response.text().await?;

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response_text) {
        if parsed.get("ok").and_then(|ok| ok.as_bool()) == Some(false) {
            let error = parsed.get("error").and_then(|e| e.as_str()).unwrap_or("unknown");
            console_log!("Channel info API error: {}", error);

            if error == "channel_not_found" {
                console_log!("✅ is_external_channel completed: {} -> true (channel not found)", channel_id);
                return Ok(true);
            }

            if error == "not_in_channel" {
                console_log!("✅ is_external_channel completed: {} -> false (not in channel)", channel_id);
                return Ok(false);
            }
        }

        if let Some(channel) = parsed.get("channel") {
            let is_ext_shared = channel.get("is_ext_shared").and_then(|v| v.as_bool()).unwrap_or(false);
            let is_shared = channel.get("is_shared").and_then(|v| v.as_bool()).unwrap_or(false);
            let is_external = is_ext_shared || is_shared;
            console_log!("✅ is_external_channel completed: {} -> {}", channel_id, is_external);
            return Ok(is_external);
        }
    }

    console_log!("✅ is_external_channel completed: {} -> true (parse error)", channel_id);
    Ok(true)
}

pub async fn get_message_history(token: &str, channel_id: &str, message_ts: &str) -> Result<MessageHistory> {
    let url = format!(
        "https://slack.com/api/conversations.history?channel={}&latest={}&limit=1&inclusive=true",
        channel_id, message_ts
    );

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", token))?;

    let request = Request::new_with_init(
        &url,
        RequestInit::new()
            .with_method(Method::Get)
            .with_headers(headers),
    )?;

    let mut response = Fetch::Request(request).send().await?;
    let response_text = response.text().await?;

    let history: MessageHistory = serde_json::from_str(&response_text)
        .map_err(|e| Error::from(format!("Failed to parse history response: {}", e)))?;

    Ok(history)
}

pub async fn get_thread_replies(token: &str, channel_id: &str, thread_ts: &str, reply_ts: &str) -> Result<MessageHistory> {
    let url = format!(
        "https://slack.com/api/conversations.replies?channel={}&ts={}",
        channel_id, thread_ts
    );

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", token))?;

    let request = Request::new_with_init(
        &url,
        RequestInit::new()
            .with_method(Method::Get)
            .with_headers(headers),
    )?;

    let mut response = Fetch::Request(request).send().await?;
    let response_text = response.text().await?;

    let mut replies: MessageHistory = serde_json::from_str(&response_text)
        .map_err(|e| Error::from(format!("Failed to parse replies response: {}", e)))?;

    // Filter to find the specific reply message
    if let Some(ref mut messages) = replies.messages {
        messages.retain(|msg| msg.ts.as_ref().map_or(false, |ts| ts == reply_ts));
        
        // Log if the specific message was not found
        if messages.is_empty() {
            console_log!("⚠️ Specific message {} not found in thread {}", reply_ts, thread_ts);
        }
    }

    Ok(replies)
}

pub async fn join_channel(token: &str, channel_id: &str) -> Result<()> {
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", token))?;
    headers.set("Content-Type", "application/json")?;

    let body = format!(r#"{{"channel":"{}"}}"#, channel_id);

    let request = Request::new_with_init(
        "https://slack.com/api/conversations.join",
        RequestInit::new()
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(body.into())),
    )?;

    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() < 300 {
        console_log!("Successfully joined channel: {}", channel_id);
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_default();
        console_log!("Failed to join channel {}: {}", channel_id, error_text);
        Err(Error::from(format!("Failed to join channel: {}", error_text)))
    }
}

pub async fn post_message(token: &str, channel: &str, text: &str) -> Result<()> {
    let slack_message = SlackMessage {
        channel: channel.to_string(),
        text: text.to_string(),
    };

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", token))?;
    headers.set("Content-Type", "application/json")?;

    let request = Request::new_with_init(
        "https://slack.com/api/chat.postMessage",
        RequestInit::new()
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(serde_json::to_string(&slack_message)?.into())),
    )?;

    let mut response = Fetch::Request(request).send().await?;
    let status_code = response.status_code();

    if status_code < 300 {
        console_log!("Message posted successfully to {}", channel);
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_default();
        console_log!("Message post failed (status: {}): {}", status_code, error_text);
        Err(Error::from(format!("Failed to post message: {}", error_text)))
    }
}

pub fn generate_message_url(workspace_domain: &str, channel_id: &str, message_ts: &str) -> String {
    let timestamp_for_url = message_ts.replace(".", "");
    format!("https://{}.slack.com/archives/{}/p{}", workspace_domain, channel_id, timestamp_for_url)
}
