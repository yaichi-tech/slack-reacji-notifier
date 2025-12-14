use std::future::Future;
use worker::*;
use crate::types::*;

/// 絵文字追加通知のユースケース
pub async fn notify_emoji_added<F, Fut>(
    emoji_name: &str,
    channel_id: &str,
    post_message: F,
) -> Result<()>
where
    F: FnOnce(&str, &str) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    console_log!("✅ New emoji added: {}", emoji_name);

    let notification = format!(":{}:  :{}:  :{}:", emoji_name, emoji_name, emoji_name);
    console_log!("📤 Posting to channel {}: {}", channel_id, notification);

    post_message(channel_id, &notification).await?;
    console_log!("🎉 EMOJI NOTIFICATION SENT SUCCESSFULLY!");

    Ok(())
}

/// リアクション追加通知のユースケース
pub async fn notify_reaction_added<F1, F2, F3, F4, F5, Fut1, Fut2, Fut3, Fut4, Fut5>(
    event: &ReactionAddedEvent,
    debug_mode: bool,
    get_user_info: F1,
    get_team_info: F2,
    is_external_channel: F3,
    get_message_author: F4,
    send_dm: F5,
) -> Result<()>
where
    F1: FnOnce(&str) -> Fut1,
    Fut1: Future<Output = Result<String>>,
    F2: FnOnce() -> Fut2,
    Fut2: Future<Output = Result<String>>,
    F3: FnOnce(&str) -> Fut3,
    Fut3: Future<Output = Result<bool>>,
    F4: FnOnce(&str, &str, Option<&str>) -> Fut4,
    Fut4: Future<Output = Result<Option<String>>>,
    F5: FnOnce(&str, &str) -> Fut5,
    Fut5: Future<Output = Result<()>>,
{
    console_log!("🚀 === REACTION EVENT PROCESSING STARTED ===");
    console_log!("Event details: user={}, reaction={}, channel={}, ts={}",
               event.user, event.reaction, event.item.channel, event.item.ts);

    // 外部チャンネルチェック
    console_log!("🔍 Checking if channel is external...");
    if let Ok(is_external) = is_external_channel(&event.item.channel).await {
        if is_external {
            console_log!("⚠️ Skipping external/Slack Connect channel: {}", event.item.channel);
            console_log!("🏁 === REACTION EVENT PROCESSING COMPLETED (EXTERNAL CHANNEL) ===");
            return Ok(());
        }
        console_log!("✅ Channel is internal, proceeding with notification...");
    } else {
        console_log!("⚠️ Could not determine if channel is external, proceeding with caution...");
    }

    // リアクターの名前を取得
    console_log!("🔄 Step 1: Getting reactor user info...");
    let reactor_name = get_user_info(&event.user).await?;

    // ワークスペースドメインを取得
    console_log!("🔄 Step 2: Getting workspace domain...");
    let workspace_domain = get_team_info().await?;

    // メッセージURLを生成
    console_log!("🔄 Step 3: Generating message URL...");
    let message_url = crate::slack::generate_message_url(&workspace_domain, &event.item.channel, &event.item.ts);
    console_log!("✅ Message URL: {}", message_url);

    // 通知メッセージを作成
    console_log!("🔄 Step 4: Creating notification message...");
    let notification = format!(
        ":{}: {}\n{}",
        event.reaction,
        reactor_name,
        message_url
    );
    console_log!("✅ Notification message created: {}", notification);

    // 元メッセージの作成者を取得
    console_log!("🔄 Step 5: Getting original message author...");
    let author = get_message_author(&event.item.channel, &event.item.ts, event.item.thread_ts.as_deref()).await?;

    match author {
        Some(author_id) => {
            console_log!("Found original message author: {}", author_id);
            let should_notify = debug_mode || author_id != event.user;
            console_log!("Should notify? {} (debug_mode: {}, same_user: {})",
                       should_notify, debug_mode, author_id == event.user);

            if should_notify {
                console_log!("🔄 Step 6: Sending notification to {}", author_id);
                send_dm(&author_id, &notification).await?;
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
    Ok(())
}

/// メッセージ作成者を取得（チャンネル自動参加のリトライロジック含む）
pub async fn get_message_author_with_retry<F1, F2, Fut1, Fut2>(
    channel_id: &str,
    message_ts: &str,
    thread_ts: Option<&str>,
    get_history: F1,
    join_channel: F2,
) -> Result<Option<String>>
where
    F1: Fn(&str, &str, Option<&str>) -> Fut1,
    Fut1: Future<Output = Result<MessageHistory>>,
    F2: FnOnce(&str) -> Fut2,
    Fut2: Future<Output = Result<()>>,
{
    console_log!("📡 Getting message history...");
    let history = get_history(channel_id, message_ts, thread_ts).await?;

    if !history.ok {
        let error = history.error.as_deref().unwrap_or("Unknown error");
        console_log!("Slack API error: {}", error);

        if error == "not_in_channel" {
            console_log!("Attempting to join channel: {}", channel_id);
            if join_channel(channel_id).await.is_ok() {
                console_log!("Successfully joined channel, retrying message history...");
                let retry_history = get_history(channel_id, message_ts, thread_ts).await?;

                if !retry_history.ok {
                    console_log!("Still failed to get message history after joining channel");
                    return Ok(None);
                }

                return Ok(extract_author_from_history(&retry_history));
            } else {
                console_log!("Failed to join channel: {}", channel_id);
            }
        }

        return Ok(None);
    }

    Ok(extract_author_from_history(&history))
}

fn extract_author_from_history(history: &MessageHistory) -> Option<String> {
    history.messages
        .as_ref()
        .and_then(|messages| messages.first())
        .and_then(|message| message.user.clone())
}
