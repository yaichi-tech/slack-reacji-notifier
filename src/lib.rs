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
    ok: bool,
    messages: Option<Vec<Message>>,
    error: Option<String>,
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

    let user_name = user_info.user
        .map(|user| user.real_name.unwrap_or(user.name))
        .unwrap_or_else(|| user_id.to_string());

    Ok(user_name)
}

async fn get_team_info(token: &str) -> Result<String> {
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
        // APIエラーチェック
        if parsed.get("ok").and_then(|ok| ok.as_bool()) == Some(false) {
            let error_msg = parsed.get("error").and_then(|e| e.as_str()).unwrap_or("unknown error");
            console_log!("Team info API failed: {}", error_msg);
        }

        // チェーンした処理でドメインを取得
        if let Some(domain) = parsed.get("team")
            .and_then(|team| team.get("domain"))
            .and_then(|domain| domain.as_str()) {
            return Ok(domain.to_string());
        }
    }

    // フォールバック: ワークスペース名が取得できない場合
    console_log!("Failed to get workspace domain, using fallback");
    Ok("yourworkspace".to_string())
fn generate_message_url(workspace_domain: &str, channel_id: &str, message_ts: &str) -> String {
    // タイムスタンプから小数点を除去してprefixを追加
    let timestamp_for_url = message_ts.replace(".", "");
    format!("https://{}.slack.com/archives/{}/p{}", workspace_domain, channel_id, timestamp_for_url)
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

    let channel_name = channel_info.channel
        .map(|channel| format!("#{}", channel.name))
        .unwrap_or_else(|| channel_id.to_string());

    Ok(channel_name)
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

    // チェーンした Option の処理
    let message_text = history.messages
        .and_then(|messages| messages.first())
        .and_then(|message| message.text.as_deref())
        .unwrap_or("(メッセージ内容なし)");

    Ok(format!("「{}」", message_text))
}

async fn join_channel(token: &str, channel_id: &str) -> Result<()> {
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
    let status_code = response.status_code();

    if status_code < 300 {
        console_log!("DM sent successfully to user {}", user_id);
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_default();
        console_log!("DM send failed (status: {}): {}", status_code, error_text);
        log_json_response("DM Error Response", &error_text);
        Err(Error::from(format!("Failed to send DM: {}", error_text)))
    }
}

fn format_timestamp(ts: &str) -> String {
    if let Ok(timestamp) = ts.parse::<f64>() {
        // Unix timestampを秒とナノ秒に分割
        let seconds = timestamp.trunc() as i64;
        let nanoseconds = ((timestamp.fract() * 1_000_000_000.0) as u32);

        // UTC時刻を取得（ナノ秒まで含む）
        let utc_datetime = chrono::DateTime::from_timestamp(seconds, nanoseconds)
            .unwrap_or_else(|| {
                console_log!("Failed to parse timestamp: {}, using default", ts);
                chrono::DateTime::from_timestamp(0, 0).unwrap()
            });

        // JST（UTC+9）に変換
        let jst_offset = chrono::FixedOffset::east_opt(9 * 3600).unwrap(); // +9時間
        let jst_datetime = utc_datetime.with_timezone(&jst_offset);

        jst_datetime.format("%Y年%m月%d日 %H:%M:%S JST").to_string()
    } else {
        console_log!("Failed to parse timestamp as f64: {}", ts);
        ts.to_string()
    }
}

fn log_json_response(label: &str, json_text: &str) {
    // Pretty print JSONを試す
    let formatted_json = serde_json::from_str::<serde_json::Value>(json_text)
        .and_then(|parsed| serde_json::to_string_pretty(&parsed))
        .unwrap_or_else(|_| json_text.to_string());

    console_log!("{}: {}", label, formatted_json);
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
            return match event.challenge {
                Some(challenge) => Response::ok(challenge),
                None => Response::error("Missing challenge", 400),
            };
        }

        // Handle reaction_added event
        if event.event_type == "event_callback" {
            let Some(reaction_event) = event.event else {
                return Response::ok("OK");
            };

            if reaction_event.event_type == "reaction_added" {
                    console_log!("Reaction added event received");

                    // Check if debug mode is enabled
                    let debug_mode = env.var("DEBUG_MODE").is_ok();

                    // Get the original message author
                    let message_info = get_message_info(
                        &slack_token,
                        &reaction_event.item.channel,
                        &reaction_event.item.ts,
                    ).await?;                    // Get user info for the person who added the reaction
                    let reactor_name = get_user_info(&slack_token, &reaction_event.user).await?;

                    // Get channel name
                    let channel_name = get_channel_name(&slack_token, &reaction_event.item.channel).await?;

                    // Get workspace domain
                    let workspace_domain = get_team_info(&slack_token).await?;

                    // Generate message URL
                    let message_url = generate_message_url(&workspace_domain, &reaction_event.item.channel, &reaction_event.item.ts);

                    // Format timestamp
                    let formatted_time = format_timestamp(&reaction_event.event_ts);

                    // Create notification message
                    let notification = format!(
                        ":{}: {}\n{}",
                        reaction_event.reaction,
                        reactor_name,
                        message_url
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
                    let response_text = response.text().await?;

                    let history: MessageHistory = serde_json::from_str(&response_text)
                        .map_err(|e| Error::from(format!("Failed to parse history response: {}", e)))?;

                    if !history.ok {
                        let error = history.error.as_deref().unwrap_or("Unknown error");
                        console_log!("Slack API error: {}", error);

                        // Try to join the channel if not in channel
                        if error == "not_in_channel" {
                            console_log!("Attempting to join channel: {}", reaction_event.item.channel);
                            if let Ok(_) = join_channel(&slack_token, &reaction_event.item.channel).await {
                                console_log!("Successfully joined channel, retrying message history...");

                                // Retry getting message history
                                let headers = Headers::new();
                                headers.set("Authorization", &format!("Bearer {}", slack_token))?;

                                let retry_request = Request::new_with_init(
                                    &history_url,
                                    RequestInit::new()
                                        .with_method(Method::Get)
                                        .with_headers(headers),
                                )?;

                                let mut retry_response = Fetch::Request(retry_request).send().await?;
                                let retry_response_text = retry_response.text().await?;

                                let retry_history: MessageHistory = serde_json::from_str(&retry_response_text)
                                    .map_err(|e| Error::from(format!("Failed to parse retry response: {}", e)))?;

                                // より綺麗な書き方：早期リターンとパターンマッチングの組み合わせ
                                if !retry_history.ok {
                                    console_log!("Still failed to get message history after joining channel");
                                    return Ok(Response::ok("OK"));
                                }

                                // チェーンした Option の処理
                                let original_author = retry_history.messages
                                    .and_then(|messages| messages.first())
                                    .and_then(|message| message.user.as_ref());

                                if let Some(author) = original_author {
                                    let should_notify = debug_mode || author != &reaction_event.user;

                                    if should_notify {
                                        send_dm(&slack_token, author, &notification).await?;
                                    }
                                }
                            } else {
                                console_log!("Failed to join channel: {}", reaction_event.item.channel);
                            }
                        }

                        console_log!("Skipped notification: Slack API returned error");
                        return Ok(Response::ok("OK"));
                    }

                    // チェーンした Option の処理で綺麗に書く
                    let original_author = history.messages
                        .and_then(|messages| messages.first())
                        .and_then(|message| message.user.as_ref());

                    match original_author {
                        Some(author) => {
                            let should_notify = debug_mode || author != &reaction_event.user;

                            if should_notify {
                                send_dm(&slack_token, author, &notification).await?;
                            }
                        }
                        None => {
                            console_log!("Skipped notification: Could not find original message author");
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
