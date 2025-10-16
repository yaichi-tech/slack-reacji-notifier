# Slack Reacji Notifier Bot

リアクションされたときにDMで通知するSlack Botです。

## 機能

- 誰かがあなたの投稿にリアクションすると、以下の情報をDMで送信します：
  - リアクションをしたユーザー名
  - リアクションした時刻
  - どの投稿に対してリアクションしたか
  - リアクションの種類

## セットアップ

### 1. Slack App の作成

1. [Slack API](https://api.slack.com/apps) にアクセス
2. "Create New App" → "From scratch"
3. App名とWorkspaceを選択

### 2. OAuth & Permissions の設定

以下のBot Token Scopesを追加：
- `channels:history` - チャンネルのメッセージ履歴を読む
- `chat:write` - メッセージを送信
- `reactions:read` - リアクション情報を読む
- `users:read` - ユーザー情報を読む
- `channels:read` - チャンネル情報を読む
- `channels:join` - チャンネルへの自動参加
- `team:read` - ワークスペース情報を読む（URL生成用）

### 3. Event Subscriptions の設定

1. Event Subscriptions を有効にする
2. Request URL: `https://your-worker-domain.workers.dev/slack/events`
3. Subscribe to bot events に `reaction_added` を追加

### 4. 環境変数の設定

```bash
# Slack Bot Tokenをシークレットに設定
wrangler secret put SLACK_BOT_TOKEN
# xoxb-で始まるBot User OAuth Tokenを入力
```

**デバッグモード（オプション）**:
開発時に自分の投稿へのリアクションでもテストしたい場合、`.dev.vars`で以下を有効化：
```toml
DEBUG_MODE = "true"
```

### 5. デプロイ

```bash
# デプロイ
npx wrangler deploy
```

### 6. Slack App のインストール

1. "Install App" でワークスペースにインストール
2. 必要なチャンネルにBotを招待

## 使用方法

1. Botがインストールされているワークスペースで投稿する
2. 他のユーザーがあなたの投稿にリアクションする  
3. 自動的にDMで通知が送信される

## 注意事項

- 自分の投稿に自分でリアクションしても通知は送られません
- Botが参加していないプライベートチャンネルでは動作しません

## 開発

```bash
# ローカルで開発サーバーを起動
npx wrangler dev

# ログを確認
npx wrangler tail
```
