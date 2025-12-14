use worker::*;
use crate::types::*;
use crate::slack;
use crate::usecases;

pub async fn handle_slack_events(req: &mut Request, env: &Env) -> Result<Response> {
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

    // Handle event_callback events
    if event.event_type == "event_callback" {
        let Some(event_data) = event.event else {
            return Response::ok("OK");
        };

        let inner_event_type = event_data.get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("");

        match inner_event_type {
            "emoji_changed" => {
                handle_emoji_changed(&event_data, &slack_token, env).await?;
            }
            "reaction_added" => {
                handle_reaction_added(&event_data, &slack_token, env).await?;
            }
            _ => {
                console_log!("Unknown event type: {}", inner_event_type);
            }
        }
    }

    console_log!("📤 Returning OK response");
    Response::ok("OK")
}

async fn handle_emoji_changed(
    event_data: &serde_json::Value,
    slack_token: &str,
    env: &Env,
) -> Result<()> {
    console_log!("🎨 === EMOJI CHANGED EVENT RECEIVED ===");

    let Ok(emoji_event) = serde_json::from_value::<EmojiChangedEvent>(event_data.clone()) else {
        console_log!("❌ Failed to parse emoji_changed event");
        return Ok(());
    };

    console_log!("Event details: subtype={}, name={}", emoji_event.subtype, emoji_event.name);

    if emoji_event.subtype != "add" {
        console_log!("⏭️ Skipping non-add emoji event: subtype={}", emoji_event.subtype);
        console_log!("🏁 === EMOJI CHANGED EVENT PROCESSING COMPLETED ===");
        return Ok(());
    }

    let Ok(notification_channel) = env.var("EMOJI_NOTIFICATION_CHANNEL") else {
        console_log!("⚠️ EMOJI_NOTIFICATION_CHANNEL not configured, skipping notification");
        console_log!("🏁 === EMOJI CHANGED EVENT PROCESSING COMPLETED ===");
        return Ok(());
    };

    let channel_id = notification_channel.to_string();
    let token = slack_token.to_string();

    // 部分適用でSlack関数をDI
    let post_message = |channel: &str, text: &str| {
        let token = token.clone();
        let channel = channel.to_string();
        let text = text.to_string();
        async move { slack::post_message(&token, &channel, &text).await }
    };

    if let Err(e) = usecases::notify_emoji_added(&emoji_event.name, &channel_id, post_message).await {
        console_log!("❌ Failed to post emoji notification: {:?}", e);
    }

    console_log!("🏁 === EMOJI CHANGED EVENT PROCESSING COMPLETED ===");
    Ok(())
}

async fn handle_reaction_added(
    event_data: &serde_json::Value,
    slack_token: &str,
    env: &Env,
) -> Result<()> {
    let Ok(reaction_event) = serde_json::from_value::<ReactionAddedEvent>(event_data.clone()) else {
        console_log!("❌ Failed to parse reaction_added event");
        return Ok(());
    };

    if reaction_event.event_type != "reaction_added" {
        return Ok(());
    }

    let debug_mode = env.var("DEBUG_MODE").is_ok();
    console_log!("Debug mode enabled: {}", debug_mode);

    let token = slack_token.to_string();

    // 部分適用でSlack関数をDI
    let get_user_info = |user_id: &str| {
        let token = token.clone();
        let user_id = user_id.to_string();
        async move { slack::get_user_info(&token, &user_id).await }
    };

    let get_team_info = || {
        let token = token.clone();
        async move { slack::get_team_info(&token).await }
    };

    let is_external_channel = |channel_id: &str| {
        let token = token.clone();
        let channel_id = channel_id.to_string();
        async move { slack::is_external_channel(&token, &channel_id).await }
    };

    let get_message_author = |channel_id: &str, message_ts: &str, thread_ts: Option<&str>| {
        let token = token.clone();
        let channel_id = channel_id.to_string();
        let message_ts = message_ts.to_string();
        let thread_ts = thread_ts.map(|s| s.to_string());
        async move {
            let get_history = |ch: &str, ts: &str, thread_ts: Option<&str>| {
                let token = token.clone();
                let ch = ch.to_string();
                let ts = ts.to_string();
                let thread_ts = thread_ts.map(|s| s.to_string());
                async move {
                    // If thread_ts is present and different from ts, this is a threaded reply
                    if let Some(ref thread_ts_val) = thread_ts {
                        if thread_ts_val != &ts {
                            console_log!("Fetching threaded message: thread_ts={}, ts={}", thread_ts_val, ts);
                            return slack::get_thread_replies(&token, &ch, thread_ts_val, &ts).await;
                        }
                    }
                    // Otherwise, fetch as regular message
                    slack::get_message_history(&token, &ch, &ts).await
                }
            };

            let join_channel = |ch: &str| {
                let token = token.clone();
                let ch = ch.to_string();
                async move { slack::join_channel(&token, &ch).await }
            };

            usecases::get_message_author_with_retry(&channel_id, &message_ts, thread_ts.as_deref(), get_history, join_channel).await
        }
    };

    let send_dm = |user_id: &str, message: &str| {
        let token = token.clone();
        let user_id = user_id.to_string();
        let message = message.to_string();
        async move { slack::post_message(&token, &user_id, &message).await }
    };

    if let Err(e) = usecases::notify_reaction_added(
        &reaction_event,
        debug_mode,
        get_user_info,
        get_team_info,
        is_external_channel,
        get_message_author,
        send_dm,
    ).await {
        console_log!("❌ Error handling reaction_added event: {:?}", e);
    }

    Ok(())
}
