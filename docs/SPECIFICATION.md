# SPECIFICATION.md - shiki 機能仕様書

> **Version**: 0.1.0  
> **Last Updated**: 2025-12-30  
> **Status**: Draft

---

## 1. 概要

### 1.1 システム目的

shiki は、複数マシン間で systemd サービスの起動順序を HTTP ベースで連携させる軽量エージェントです。

### 1.2 適用範囲

- Linux 環境（systemd 必須）
- ホスト OS 上での直接実行
- Docker コンテナ内での実行

### 1.3 前提条件

- systemd が動作していること
- ネットワーク経由で他のエージェントと通信可能であること
- 対象サービスの操作権限があること（systemctl の実行権限）

---

## 2. エージェント仕様

### 2.1 動作モード

| モード | 説明 | 用途 |
|--------|------|------|
| `serve` | HTTP サーバーとして常駐 | 通知を受信し、ローカルサービスを操作 |
| `notify` | 他エージェントへ通知送信 | ExecStartPre 等から呼び出し |
| `status` | エージェント状態確認 | ヘルスチェック・デバッグ |
| `config` | 設定ファイル操作 | 検証・表示 |

### 2.2 CLI インターフェース

```
shiki [OPTIONS] <COMMAND>

COMMANDS:
    serve     HTTP サーバーを起動し、通知を待ち受ける
    notify    リモートエージェントへ通知を送信する
    status    エージェントまたはサービスの状態を確認する
    config    設定ファイルの検証・表示を行う
    help      ヘルプを表示する

OPTIONS:
    -c, --config <PATH>    設定ファイルパス [default: /etc/shiki/config.yaml]
    -v, --verbose          詳細ログ出力（複数指定で増加: -vv, -vvv）
    -q, --quiet            エラーのみ出力
    -h, --help             ヘルプを表示
    -V, --version          バージョンを表示
```

### 2.3 サブコマンド詳細

#### `shiki serve`

```
shiki serve [OPTIONS]

OPTIONS:
    --bind <ADDR>     バインドアドレス [default: 0.0.0.0]
    --port <PORT>     リッスンポート [default: 8080]
```

#### `shiki notify`

```
shiki notify [OPTIONS] --target <TARGET> --action <ACTION> --service <SERVICE>

OPTIONS:
    -t, --target <TARGET>      通知先アドレス (host:port)
    -a, --action <ACTION>      アクション (start|stop|restart)
    -s, --service <SERVICE>    対象サービス名
    -w, --wait                 完了まで待機 [default: true]
    --timeout <SECONDS>        タイムアウト秒数 [default: 60]
    --no-wait                  完了を待たない
```

#### `shiki status`

```
shiki status [OPTIONS]

OPTIONS:
    --target <TARGET>     リモートエージェントの状態を確認
    --service <SERVICE>   サービス状態を確認（ローカル）
```

#### `shiki config`

```
shiki config <SUBCOMMAND>

SUBCOMMANDS:
    validate    設定ファイルを検証する
    show        現在の設定を表示する
```

---

## 3. エージェントライフサイクル

### 3.1 状態遷移図

```mermaid
stateDiagram-v2
    [*] --> Initializing: shiki serve
    
    Initializing --> Ready: 設定読み込み完了
    Initializing --> Failed: 設定エラー
    
    Ready --> Processing: リクエスト受信
    Processing --> Ready: リクエスト完了
    Processing --> Error: 処理失敗
    
    Error --> Ready: 自動復旧
    Error --> Failed: 復旧不可
    
    Ready --> Shutdown: SIGTERM/SIGINT
    Shutdown --> [*]: 終了
    
    Failed --> [*]: 異常終了
```

### 3.2 状態一覧

| 状態 | 説明 | HTTP レスポンス |
|------|------|-----------------|
| `Initializing` | 起動中、設定読み込み中 | 503 Service Unavailable |
| `Ready` | リクエスト受付可能 | 200 OK |
| `Processing` | リクエスト処理中 | 200 OK |
| `Error` | 一時的エラー、復旧待ち | 503 Service Unavailable |
| `Shutdown` | シャットダウン中 | 503 Service Unavailable |
| `Failed` | 致命的エラー、終了待ち | N/A（プロセス終了） |

---

## 4. 通知フロー仕様

### 4.1 通知トリガー条件

| トリガー | 説明 | 使用例 |
|----------|------|--------|
| **ExecStartPre** | サービス起動前に実行 | 依存サービスの起動を待つ |
| **ExecStartPost** | サービス起動後に実行 | 依存先に起動完了を通知 |
| **ExecStopPost** | サービス停止後に実行 | 依存先に停止を通知 |
| **手動実行** | CLI から直接実行 | テスト・デバッグ |

### 4.2 通知シーケンス（成功時）

```mermaid
sequenceDiagram
    participant Caller as 呼び出し元
    participant Sender as shiki notify
    participant Receiver as shiki serve (リモート)
    participant systemd as systemd (リモート)
    
    Caller->>Sender: shiki notify --target ... --action start --service nginx
    Sender->>Receiver: POST /api/v1/notify
    Note over Receiver: リクエスト検証
    Receiver->>systemd: systemctl start nginx
    
    alt wait=true
        systemd-->>Receiver: サービス起動完了
        Receiver-->>Sender: 200 OK {status: "started"}
    else wait=false
        Receiver-->>Sender: 202 Accepted {status: "pending"}
    end
    
    Sender-->>Caller: Exit 0
```

### 4.3 通知シーケンス（失敗・リトライ時）

```mermaid
sequenceDiagram
    participant Sender as shiki notify
    participant Receiver as shiki serve (リモート)
    
    Sender->>Receiver: POST /api/v1/notify (1回目)
    Receiver-->>Sender: 503 Service Unavailable
    
    Note over Sender: 1秒待機
    Sender->>Receiver: POST /api/v1/notify (2回目)
    Receiver-->>Sender: 503 Service Unavailable
    
    Note over Sender: 2秒待機 (backoff)
    Sender->>Receiver: POST /api/v1/notify (3回目)
    Receiver-->>Sender: 200 OK
    
    Sender-->>Sender: Exit 0
```

### 4.4 リトライポリシー

| パラメータ | デフォルト値 | 説明 |
|------------|-------------|------|
| `max_retries` | 3 | 最大リトライ回数 |
| `retry_delay_ms` | 1000 | 初回リトライ遅延（ミリ秒） |
| `backoff_factor` | 2.0 | 指数バックオフ係数 |
| `max_delay_ms` | 30000 | 最大リトライ遅延（ミリ秒） |

**リトライ遅延計算式:**

```
delay = min(retry_delay_ms * (backoff_factor ^ attempt), max_delay_ms)
```

### 4.5 タイムアウト仕様

| パラメータ | デフォルト値 | 説明 |
|------------|-------------|------|
| `connect_timeout_seconds` | 5 | TCP 接続タイムアウト |
| `read_timeout_seconds` | 30 | HTTP レスポンス読み取りタイムアウト |
| `service_timeout_seconds` | 60 | サービス起動/停止待機タイムアウト |

---

## 5. サービス操作仕様

### 5.1 対応アクション

| アクション | systemctl コマンド | 説明 |
|------------|-------------------|------|
| `start` | `systemctl start <service>` | サービスを起動 |
| `stop` | `systemctl stop <service>` | サービスを停止 |
| `restart` | `systemctl restart <service>` | サービスを再起動 |
| `status` | `systemctl is-active <service>` | 状態を確認（操作なし） |

### 5.2 サービス状態

| 状態 | 説明 |
|------|------|
| `running` | サービスが稼働中 |
| `stopped` | サービスが停止中 |
| `failed` | サービスが異常終了 |
| `unknown` | 状態不明（サービス未登録等） |

### 5.3 サービスアクセス制御

設定ファイルで許可/拒否リストを定義可能：

```yaml
services:
  allowed:
    - nginx
    - postgresql
    - redis
  denied:
    - sshd
    - systemd-*
```

**評価順序:**
1. `denied` リストに一致 → 拒否
2. `allowed` リストが空 → 全許可
3. `allowed` リストに一致 → 許可
4. それ以外 → 拒否

---

## 6. エラーコード仕様

### 6.1 エラーコード一覧

| コード | 名前 | HTTP Status | 説明 |
|--------|------|-------------|------|
| `E001` | `CONFIG_INVALID` | - | 設定ファイルが不正 |
| `E002` | `SERVICE_NOT_FOUND` | 404 | 対象サービスが存在しない |
| `E003` | `SERVICE_DENIED` | 403 | サービス操作が許可されていない |
| `E004` | `SYSTEMD_ERROR` | 500 | systemd 操作に失敗 |
| `E005` | `TIMEOUT` | 504 | 操作がタイムアウト |
| `E006` | `CONNECTION_ERROR` | 502 | リモートエージェントへの接続失敗 |
| `E007` | `AUTH_FAILED` | 401 | 認証失敗 |
| `E008` | `INVALID_REQUEST` | 400 | リクエストが不正 |
| `E009` | `AGENT_BUSY` | 503 | エージェントがビジー状態 |

### 6.2 エラーレスポンス形式

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "E002",
    "message": "Service not found: nginx",
    "details": {
      "service": "nginx",
      "suggestion": "Check if the service is installed and the name is correct"
    }
  },
  "timestamp": "2025-12-30T10:00:00Z"
}
```

---

## 7. 終了コード

CLI 実行時の終了コード：

| コード | 説明 |
|--------|------|
| 0 | 成功 |
| 1 | 一般的なエラー |
| 2 | 設定エラー |
| 3 | 接続エラー |
| 4 | タイムアウト |
| 5 | 認証エラー |
| 64 | コマンドライン引数エラー |

---

## 8. 制約事項

### 8.1 動作要件

| 項目 | 要件 |
|------|------|
| OS | Linux（systemd 必須） |
| アーキテクチャ | x86_64 / aarch64 / armv7 |
| Rust バージョン | 1.70 以上（ビルド時） |
| systemd バージョン | 219 以上 |

### 8.2 制限値

| 項目 | 制限値 | 変更可否 |
|------|--------|----------|
| 最大同時接続数 | 100 | 設定可能 |
| リクエストボディ最大サイズ | 1 MB | 固定 |
| サービス名最大長 | 256 文字 | 固定 |
| 管理対象サービス数 | 無制限 | - |
| 同時処理リクエスト数 | 10 | 設定可能 |

---

## 関連ドキュメント

- [DESIGN.md](DESIGN.md) - アーキテクチャ設計書
- [API.md](API.md) - REST API リファレンス
- [CONFIGURATION.md](CONFIGURATION.md) - 設定リファレンス
- [SYSTEMD_INTEGRATION.md](SYSTEMD_INTEGRATION.md) - systemd 連携ガイド
