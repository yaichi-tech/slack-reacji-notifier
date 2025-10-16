use worker::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
struct SlackEvent {
    #[serde(rename = "type")]
    event_type: String,
    challenge: Option<String>,
    event: Option<ReactionAddedEvent>,
}

#[derive(Deserialize, Debug)]
struct ReactionAddedEvent {
    #[serde(rename = "type")]
    event_type: String,
    user: String,
    item: EventItem,
    reaction: String,
    #[serde(rename = "event_ts")]
    event_ts: String,
}

#[derive(Deserialize, Debug)]
struct EventItem {
    channel: String,
    ts: String,
}

#[derive(Serialize)]
struct SlackMessage {
    channel: String,
    text: String,
}

#[derive(Deserialize)]
struct ChannelInfo {
    channel: Option<Channel>,
}

#[derive(Deserialize)]
struct Channel {
    name: String,
}

#[derive(Deserialize)]
struct MessageHistory {
    messages: Option<Vec<Message>>,
}

#[derive(Deserialize)]
struct Message {
    text: Option<String>,
    user: Option<String>,
}

#[derive(Deserialize)]
struct UserInfo {
    user: Option<User>,
}

#[derive(Deserialize)]
struct User {
    name: String,
    #[serde(rename = "real_name")]
    real_name: Option<String>,
}

async fn get_user_info(token: &str, user_id: &str) -> Result<String> {
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
    let user_info: UserInfo = response.json().await?;

    if let Some(user) = user_info.user {
        Ok(user.real_name.unwrap_or(user.name))
    } else {
        Ok(user_id.to_string())
    }
}

async fn get_channel_name(token: &str, channel_id: &str) -> Result<String> {
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
    let channel_info: ChannelInfo = response.json().await?;

    if let Some(channel) = channel_info.channel {
        Ok(format!("#{}", channel.name))
    } else {
        Ok(channel_id.to_string())
    }
}

async fn get_message_info(token: &str, channel_id: &str, ts: &str) -> Result<String> {
    let url = format!(
        "https://slack.com/api/conversations.history?channel={}&latest={}&limit=1&inclusive=true",
        channel_id, ts
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
    let history: MessageHistory = response.json().await?;

    if let Some(messages) = history.messages {
        if let Some(message) = messages.first() {
            let text = message.text.as_deref().unwrap_or("(メッセージ内容なし)");
            return Ok(format!("「{}」", text));
        }
    }

    Ok("(メッセージが見つかりません)".to_string())
}

async fn send_dm(token: &str, user_id: &str, message: &str) -> Result<()> {
    let dm_message = SlackMessage {
        channel: user_id.to_string(),
        text: message.to_string(),
    };

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", token))?;
    headers.set("Content-Type", "application/json")?;

    let request = Request::new_with_init(
        "https://slack.com/api/chat.postMessage",
        RequestInit::new()
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(serde_json::to_string(&dm_message)?.into())),
    )?;

    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() < 300 {
        console_log!("DM sent successfully to user {}", user_id);
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(Error::from(format!("Failed to send DM: {}", error_text)))
    }
}

fn format_timestamp(ts: &str) -> String {
    if let Ok(timestamp) = ts.parse::<f64>() {
        let datetime = chrono::DateTime::from_timestamp(timestamp as i64, 0)
            .unwrap_or_default();
        datetime.format("%Y年%m月%d日 %H:%M:%S").to_string()
    } else {
        ts.to_string()
    }
}

#[event(fetch)]
async fn fetch(mut req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let url = req.url()?;

    if url.path() == "/slack/events" {
        if req.method() != Method::Post {
            return Response::error("Method not allowed", 405);
        }

        let slack_token = env.secret("SLACK_BOT_TOKEN")?.to_string();

        let body = req.text().await?;
        let event: SlackEvent = serde_json::from_str(&body)
            .map_err(|e| Error::from(format!("Failed to parse JSON: {}", e)))?;

        // URL verification challenge
        if event.event_type == "url_verification" {
            if let Some(challenge) = event.challenge {
                return Response::ok(challenge);
            }
        }

        // Handle reaction_added event
        if event.event_type == "event_callback" {
            if let Some(reaction_event) = event.event {
                if reaction_event.event_type == "reaction_added" {
                    console_log!("Reaction added event received");

                    // Get the original message author
                    let message_info = get_message_info(
                        &slack_token,
                        &reaction_event.item.channel,
                        &reaction_event.item.ts,
                    ).await?;

                    // Get user info for the person who added the reaction
                    let reactor_name = get_user_info(&slack_token, &reaction_event.user).await?;

                    // Get channel name
                    let channel_name = get_channel_name(&slack_token, &reaction_event.item.channel).await?;

                    // Format timestamp
                    let formatted_time = format_timestamp(&reaction_event.event_ts);

                    // Create notification message
                    let notification = format!(
                        "🎉 リアクション通知\n\n✨ {}さんがあなたの投稿にリアクションしました\n⏰ 時刻: {}\n📍 チャンネル: {}\n💬 投稿内容: {}\n🎭 リアクション: :{}:",
                        reactor_name,
                        formatted_time,
                        channel_name,
                        message_info,
                        reaction_event.reaction
                    );

                    // Get the original message to find who posted it
                    let history_url = format!(
                        "https://slack.com/api/conversations.history?channel={}&latest={}&limit=1&inclusive=true",
                        reaction_event.item.channel, reaction_event.item.ts
                    );

                    let headers = Headers::new();
                    headers.set("Authorization", &format!("Bearer {}", slack_token))?;

                    let request = Request::new_with_init(
                        &history_url,
                        RequestInit::new()
                            .with_method(Method::Get)
                            .with_headers(headers),
                    )?;

                    let mut response = Fetch::Request(request).send().await?;
                    let history: MessageHistory = response.json().await?;

                    if let Some(messages) = history.messages {
                        if let Some(message) = messages.first() {
                            if let Some(original_author) = &message.user {
                                // Don't send notification if the user reacted to their own message
                                if original_author != &reaction_event.user {
                                    send_dm(&slack_token, original_author, &notification).await?;
                                }
                            }
                        }
                    }
                }
            }
        }

        Response::ok("OK")
    } else if url.path() == "/" {
        Response::ok("Slack Reacji Notifier is running!")
    } else {
        Response::error("Not found", 404)
    }
}
