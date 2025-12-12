# Slack Reacji Notifier Bot

A Slack bot that sends notifications about reactions and emoji events.

## Features

### 1. Reaction Notifications
When someone reacts to your message, you'll receive a DM notification with:
- The reaction emoji
- The name of the user who reacted
- A link to the original message

### 2. Emoji Registration Notifications
When a new custom emoji is added to the workspace, a notification is sent to a designated channel:
- Notification format: `:new_emoji:  :new_emoji:  :new_emoji:`

---

## Setup

### 1. Create a Slack App

1. Go to [Slack API](https://api.slack.com/apps)
2. Click "Create New App" → "From scratch"
3. Choose an App name and Workspace

### 2. Configure OAuth & Permissions

Add the following Bot Token Scopes:

| Scope | Purpose |
|-------|---------|
| `channels:history` | Read channel message history |
| `chat:write` | Send messages |
| `reactions:read` | Read reaction information |
| `users:read` | Read user information |
| `channels:read` | Read channel information |
| `channels:join` | Auto-join channels |
| `team:read` | Read workspace information |
| `emoji:read` | Read emoji information (for emoji notifications) |

### 3. Configure Event Subscriptions

1. Enable Event Subscriptions
2. Request URL: `https://your-worker-domain.workers.dev/slack/events`
3. Add the following to "Subscribe to bot events":
   - `reaction_added` - For reaction notifications
   - `emoji_changed` - For emoji registration notifications

### 4. Install the App

1. Go to "Install App" and install to your workspace
2. Copy the **Bot User OAuth Token** (starts with `xoxb-`)

---

## Deployment

### Step 1: Set Slack Bot Token

```bash
wrangler secret put SLACK_BOT_TOKEN
# Enter the xoxb-... token at the prompt
```

### Step 2: Set Emoji Notification Channel (Optional)

To enable emoji registration notifications:

```bash
wrangler secret put EMOJI_NOTIFICATION_CHANNEL
# Enter the channel ID (e.g., C0123456789) at the prompt
```

How to find the channel ID:
1. Right-click on the channel in Slack → "View channel details"
2. Copy the "Channel ID" at the bottom

### Step 3: Deploy

```bash
npx wrangler deploy
```

### Step 4: Invite the Bot to a Channel

Invite the bot to the notification channel (e.g., `#reacji-release`):
```
/invite @your-bot-name
```

---

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `SLACK_BOT_TOKEN` | Yes | Slack Bot Token (`xoxb-...`) |
| `EMOJI_NOTIFICATION_CHANNEL` | No | Channel ID for emoji notifications |
| `DEBUG_MODE` | No | Set to `true` to notify self-reactions |

---

## Development

### Local Development

```bash
# Set development environment variables in .dev.vars
cat > .dev.vars << EOF
SLACK_BOT_TOKEN=xoxb-your-dev-token
EMOJI_NOTIFICATION_CHANNEL=C0123456789
DEBUG_MODE=true
EOF

# Start the development server
npx wrangler dev
```

### View Logs

```bash
npx wrangler tail
```

---

## Notes

- You won't receive notifications for reactions on your own messages (unless DEBUG_MODE=true)
- The bot won't work in private channels it hasn't joined
- The bot doesn't work in Slack Connect external channels
