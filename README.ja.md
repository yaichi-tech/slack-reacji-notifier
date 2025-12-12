# Slack Reacji Notifier Bot

Slackのリアクション・絵文字に関する通知を行うBotです。

## 機能

### 1. リアクション通知
誰かがあなたの投稿にリアクションすると、DMで通知します：
- リアクションの絵文字
- リアクションしたユーザー名
- 元の投稿へのリンク

### 2. 絵文字登録通知
新しいカスタム絵文字がワークスペースに追加されると、指定チャンネルに通知します：
- 通知形式: `:new_emoji:  :new_emoji:  :new_emoji:`

---

## セットアップ

### 1. Slack App の作成

1. [Slack API](https://api.slack.com/apps) にアクセス
2. "Create New App" → "From scratch"
3. App名とWorkspaceを選択

### 2. OAuth & Permissions の設定

以下のBot Token Scopesを追加：

| スコープ | 用途 |
|---------|------|
| `channels:history` | チャンネルのメッセージ履歴を読む |
| `chat:write` | メッセージを送信 |
| `reactions:read` | リアクション情報を読む |
| `users:read` | ユーザー情報を読む |
| `channels:read` | チャンネル情報を読む |
| `channels:join` | チャンネルへの自動参加 |
| `team:read` | ワークスペース情報を読む |
| `emoji:read` | 絵文字情報を読む（絵文字登録通知用） |

### 3. Event Subscriptions の設定

1. Event Subscriptions を有効にする
2. Request URL: `https://your-worker-domain.workers.dev/slack/events`
3. Subscribe to bot events に以下を追加：
   - `reaction_added` - リアクション通知用
   - `emoji_changed` - 絵文字登録通知用

### 4. アプリをインストール

1. "Install App" でワークスペースにインストール
2. `xoxb-` で始まる **Bot User OAuth Token** をコピー

---

## デプロイ手順

### Step 1: Slack Bot Token を設定

```bash
wrangler secret put SLACK_BOT_TOKEN
# プロンプトで xoxb-... のトークンを入力
```

### Step 2: 絵文字通知チャンネルを設定（オプション）

絵文字登録通知を有効にする場合：

```bash
wrangler secret put EMOJI_NOTIFICATION_CHANNEL
# プロンプトでチャンネルID（例: C0123456789）を入力
```

チャンネルIDの確認方法：
1. Slackでチャンネルを右クリック → "チャンネル詳細を表示"
2. 最下部の "チャンネルID" をコピー

### Step 3: デプロイ

```bash
npx wrangler deploy
```

### Step 4: Botをチャンネルに招待

通知先チャンネル（例: `#reacji-release`）にBotを招待：
```
/invite @your-bot-name
```

---

## 環境変数一覧

| 変数名 | 必須 | 説明 |
|-------|------|------|
| `SLACK_BOT_TOKEN` | Yes | Slack Bot Token (`xoxb-...`) |
| `EMOJI_NOTIFICATION_CHANNEL` | No | 絵文字通知先チャンネルID |
| `DEBUG_MODE` | No | `true` で自己リアクションも通知 |

---

## 開発

### ローカル開発

```bash
# .dev.vars に開発用の環境変数を設定
cat > .dev.vars << EOF
SLACK_BOT_TOKEN=xoxb-your-dev-token
EMOJI_NOTIFICATION_CHANNEL=C0123456789
DEBUG_MODE=true
EOF

# 開発サーバーを起動
npx wrangler dev
```

### ログ確認

```bash
npx wrangler tail
```

---

## 注意事項

- 自分の投稿に自分でリアクションしても通知は送られません（DEBUG_MODE=true の場合を除く）
- Botが参加していないプライベートチャンネルでは動作しません
- Slack Connectの外部チャンネルでは動作しません
