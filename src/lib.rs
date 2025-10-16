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
struct MessageHistory {
    ok: bool,
    messages: Option<Vec<Message>>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct Message {
    user: Option<String>,
}

#[derive(Deserialize)]
struct ChannelInfo {
    channel: Option<ChannelDetails>,
}

#[derive(Deserialize)]
struct ChannelDetails {
    name: String,
    is_ext_shared: Option<bool>,
    is_shared: Option<bool>,
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

    console_log!("✅ get_user_info completed: {} -> {}", user_id, user_name);
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
            console_log!("✅ get_team_info completed: domain = {}", domain);
            return Ok(domain.to_string());
        }
    }

    // フォールバック: ワークスペース名が取得できない場合
    console_log!("Failed to get workspace domain, using fallback");
    console_log!("✅ get_team_info completed: fallback domain = yourworkspace");
    Ok("yourworkspace".to_string())
}

async fn is_external_channel(token: &str, channel_id: &str) -> Result<bool> {
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
        // APIエラーチェック
        if parsed.get("ok").and_then(|ok| ok.as_bool()) == Some(false) {
            let error = parsed.get("error").and_then(|e| e.as_str()).unwrap_or("unknown");
            console_log!("Channel info API error: {}", error);

            // channel_not_found は外部チャンネルの可能性が高い
            // not_in_channel は内部チャンネルでもあり得るので、メイン処理での自動参加に委ねる
            if error == "channel_not_found" {
                console_log!("✅ is_external_channel completed: {} -> true (channel not found, likely external)", channel_id);
                return Ok(true);
            }

            // not_in_channel の場合は内部チャンネルとして扱い、メイン処理で自動参加を試行
            if error == "not_in_channel" {
                console_log!("✅ is_external_channel completed: {} -> false (not in channel, but may be internal)", channel_id);
                return Ok(false);
            }
        }

        // 正常にチャンネル情報が取得できた場合
        if let Some(channel) = parsed.get("channel") {
            let is_ext_shared = channel.get("is_ext_shared").and_then(|v| v.as_bool()).unwrap_or(false);
            let is_shared = channel.get("is_shared").and_then(|v| v.as_bool()).unwrap_or(false);
            let is_external = is_ext_shared || is_shared;
            console_log!("✅ is_external_channel completed: {} -> {} (is_ext_shared: {}, is_shared: {})",
                        channel_id, is_external, is_ext_shared, is_shared);
            return Ok(is_external);
        }
    }

    // パースエラーやその他のエラーの場合、安全のため外部チャンネルとして扱う
    console_log!("✅ is_external_channel completed: {} -> true (parse error, treating as external for safety)", channel_id);
    Ok(true)
}

fn generate_message_url(workspace_domain: &str, channel_id: &str, message_ts: &str) -> String {
    // タイムスタンプから小数点を除去してprefixを追加
    let timestamp_for_url = message_ts.replace(".", "");
    let url = format!("https://{}.slack.com/archives/{}/p{}", workspace_domain, channel_id, timestamp_for_url);
    console_log!("✅ generate_message_url completed: {}", url);
    url
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
                console_log!("🚀 === REACTION EVENT PROCESSING STARTED ===");
                console_log!("Reaction added event received");
                console_log!("Event details: user={}, reaction={}, channel={}, ts={}",
                           reaction_event.user, reaction_event.reaction,
                           reaction_event.item.channel, reaction_event.item.ts);

                // 外部チャンネル（Slack Connect）をチェック - C094K18T5LZ のようなパターン
                console_log!("📋 Channel ID pattern: {}", reaction_event.item.channel);

                // より確実な外部チャンネルチェック
                console_log!("🔍 Checking if channel is external...");
                if let Ok(is_external) = is_external_channel(&slack_token, &reaction_event.item.channel).await {
                    if is_external {
                        console_log!("⚠️ Skipping external/Slack Connect channel: {}", reaction_event.item.channel);
                        console_log!("🏁 === REACTION EVENT PROCESSING COMPLETED (EXTERNAL CHANNEL) ===");
                        return Response::ok("OK");
                    } else {
                        console_log!("✅ Channel is internal, proceeding with notification...");
                    }
                } else {
                    console_log!("⚠️ Could not determine if channel is external, proceeding with caution...");
                }

                // Check if debug mode is enabled
                let debug_mode = env.var("DEBUG_MODE").is_ok();
                console_log!("Debug mode enabled: {}", debug_mode);

                // Get user info for the person who added the reaction
                console_log!("🔄 Step 1: Getting reactor user info...");
                let reactor_name = get_user_info(&slack_token, &reaction_event.user).await?;

                // Get workspace domain
                console_log!("🔄 Step 2: Getting workspace domain...");
                let workspace_domain = get_team_info(&slack_token).await?;

                // Generate message URL
                console_log!("🔄 Step 3: Generating message URL...");
                let message_url = generate_message_url(&workspace_domain, &reaction_event.item.channel, &reaction_event.item.ts);

                // Create notification message
                console_log!("🔄 Step 4: Creating notification message...");
                let notification = format!(
                    ":{}: {}\n{}",
                    reaction_event.reaction,
                    reactor_name,
                    message_url
                );
                console_log!("✅ Notification message created: {}", notification);

                // Get the original message to find who posted it
                console_log!("🔄 Step 5: Getting message history to find original author...");
                let history_url = format!(
                    "https://slack.com/api/conversations.history?channel={}&latest={}&limit=1&inclusive=true",
                    reaction_event.item.channel, reaction_event.item.ts
                );
                console_log!("📡 API Request URL: {}", history_url);

                let headers = Headers::new();
                headers.set("Authorization", &format!("Bearer {}", slack_token))?;

                let request = Request::new_with_init(
                    &history_url,
                    RequestInit::new()
                        .with_method(Method::Get)
                        .with_headers(headers),
                )?;

                console_log!("📡 Sending history API request...");
                let mut response = Fetch::Request(request).send().await?;
                let response_text = response.text().await?;

                let history: MessageHistory = serde_json::from_str(&response_text)
                    .map_err(|e| Error::from(format!("Failed to parse history response: {}", e)))?;
                console_log!("✅ History API response parsed successfully");

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
                                return Response::ok("OK");
                            }

                            // チェーンした Option の処理
                            console_log!("Processing retry message history: messages count = {}",
                                       retry_history.messages.as_ref().map(|m| m.len()).unwrap_or(0));

                            let original_author = retry_history.messages
                                .as_ref()
                                .and_then(|messages| messages.first())
                                .and_then(|message| message.user.as_ref());

                            if let Some(author) = original_author {
                                console_log!("Found original message author after retry: {}", author);
                                let should_notify = debug_mode || author != &reaction_event.user;
                                console_log!("Should notify after retry? {} (debug_mode: {}, same_user: {})",
                                           should_notify, debug_mode, author == &reaction_event.user);

                                if should_notify {
                                    console_log!("🔄 Step 6 (Retry): Sending notification to {} after retry", author);
                                    send_dm(&slack_token, author, &notification).await?;
                                    console_log!("🎉 NOTIFICATION SENT SUCCESSFULLY AFTER RETRY!");
                                } else {
                                    console_log!("⏭️ Skipped notification after retry: User reacted to their own message");
                                }
                            } else {
                                console_log!("No original author found after retry");
                            }
                        } else {
                            console_log!("Failed to join channel: {}", reaction_event.item.channel);
                        }
                    }

                    console_log!("Skipped notification: Slack API returned error");
                    return Response::ok("OK");
                }

                // チェーンした Option の処理で綺麗に書く
                console_log!("Processing message history: messages count = {}",
                           history.messages.as_ref().map(|m| m.len()).unwrap_or(0));

                let original_author = history.messages
                    .as_ref()
                    .and_then(|messages| messages.first())
                    .and_then(|message| message.user.as_ref());

                match original_author {
                    Some(author) => {
                        console_log!("Found original message author: {}", author);
                        let should_notify = debug_mode || author != &reaction_event.user;
                        console_log!("Should notify? {} (debug_mode: {}, same_user: {})",
                                   should_notify, debug_mode, author == &reaction_event.user);

                        if should_notify {
                            console_log!("🔄 Step 6: Sending notification to {}", author);
                            send_dm(&slack_token, author, &notification).await?;
                            console_log!("🎉 NOTIFICATION SENT SUCCESSFULLY!");
                        } else {
                            console_log!("⏭️ Skipped notification: User reacted to their own message");
                        }
                    }
                    None => {
                        console_log!("❌ Skipped notification: Could not find original message author");
                    }
                }
                console_log!("🏁 === REACTION EVENT PROCESSING COMPLETED ===");
            }
        }

        console_log!("📤 Returning OK response");
        Response::ok("OK")
    } else if url.path() == "/" {
        Response::ok("Slack Reacji Notifier is running!")
    } else {
        Response::error("Not found", 404)
    }
}
