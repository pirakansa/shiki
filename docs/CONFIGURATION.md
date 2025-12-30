# CONFIGURATION.md - shiki 設定リファレンス

> **Version**: 0.2.0  
> **Last Updated**: 2025-12-30  
> **Status**: Draft

---

## 1. 設定ファイルパス

設定ファイルは以下の優先順位で読み込まれます：

1. コマンドライン引数: `-c, --config <PATH>`
2. 環境変数: `SHIKI_CONFIG`
3. デフォルトパス: `/etc/shiki/config.yaml`

```bash
# コマンドライン指定
shiki serve -c /path/to/config.yaml

# 環境変数指定
export SHIKI_CONFIG=/path/to/config.yaml
shiki serve
```

---

## 2. 設定ファイル全体像

```yaml
# /etc/shiki/config.yaml

# HTTP サーバー設定
server:
  bind: "0.0.0.0"
  port: 8080
  tls:
    enabled: false
    cert_path: "/etc/shiki/certs/server.crt"
    key_path: "/etc/shiki/certs/server.key"

# 認証設定
auth:
  enabled: false
  method: "token"
  token: ""

# ログ設定
logging:
  level: "info"
  format: "json"
  output: "stdout"
  file_path: "/var/log/shiki/shiki.log"

# エージェント設定
agent:
  name: ""  # 空の場合はホスト名を使用
  mode: "standalone"
  tags: []
  backend: "systemd"  # "systemd" または "exec"

# exec バックエンド用サービス定義（backend: exec の場合）
# services:
#   nginx:
#     start: "/usr/sbin/nginx"
#     stop: "/usr/sbin/nginx -s quit"
#     status: "pgrep -x nginx"

# リトライ設定
retry:
  max_attempts: 3
  delay_ms: 1000
  backoff_factor: 2.0
  max_delay_ms: 30000

# タイムアウト設定
timeout:
  connect_seconds: 5
  read_seconds: 30
  service_seconds: 60

# サービスアクセス制御（systemd バックエンド用）
acl:
  allowed: []  # 空の場合は全サービス許可
  denied: []

# クラスタ設定（将来実装）
cluster:
  enabled: false
  peers: []
```

---

## 3. 設定項目詳細

### 3.1 server - HTTP サーバー設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `bind` | string | `"0.0.0.0"` | バインドアドレス |
| `port` | integer | `8080` | リッスンポート |

#### 3.1.1 server.tls - TLS 設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `enabled` | boolean | `false` | TLS 有効化 |
| `cert_path` | string | - | サーバー証明書パス |
| `key_path` | string | - | 秘密鍵パス |

**例: TLS 有効化**

```yaml
server:
  bind: "0.0.0.0"
  port: 8443
  tls:
    enabled: true
    cert_path: "/etc/shiki/certs/server.crt"
    key_path: "/etc/shiki/certs/server.key"
```

---

### 3.2 auth - 認証設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `enabled` | boolean | `false` | 認証有効化 |
| `method` | string | `"token"` | 認証方式（`token` / `mtls`） |
| `token` | string | `""` | Bearer トークン（`method: token` 時） |

**例: トークン認証有効化**

```yaml
auth:
  enabled: true
  method: "token"
  token: "your-secret-token-here"
```

> **セキュリティ注意**: トークンは環境変数 `SHIKI_AUTH_TOKEN` での指定を推奨します。

---

### 3.3 logging - ログ設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `level` | string | `"info"` | ログレベル |
| `format` | string | `"json"` | 出力形式 |
| `output` | string | `"stdout"` | 出力先 |
| `file_path` | string | - | ファイル出力時のパス |

#### ログレベル

| レベル | 説明 |
|--------|------|
| `trace` | 最も詳細（開発・デバッグ用） |
| `debug` | デバッグ情報 |
| `info` | 通常運用情報 |
| `warn` | 警告 |
| `error` | エラーのみ |

#### 出力形式

| 形式 | 説明 |
|------|------|
| `json` | 構造化 JSON ログ（推奨） |
| `text` | 人間可読なテキスト形式 |

#### 出力先

| 出力先 | 説明 |
|--------|------|
| `stdout` | 標準出力 |
| `stderr` | 標準エラー出力 |
| `file` | ファイル（`file_path` 必須） |

**例: ファイル出力**

```yaml
logging:
  level: "info"
  format: "json"
  output: "file"
  file_path: "/var/log/shiki/shiki.log"
```

---

### 3.4 agent - エージェント設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `name` | string | ホスト名 | エージェント識別名 |
| `mode` | string | `"standalone"` | 動作モード |
| `tags` | array[string] | `[]` | タグ（フィルタリング用） |
| `backend` | string | `"systemd"` | バックエンド種別 |

#### 動作モード

| モード | 説明 |
|--------|------|
| `standalone` | 単独動作（デフォルト） |
| `cluster` | クラスタモード（将来実装） |

#### バックエンド種別

| バックエンド | 説明 |
|--------------|------|
| `systemd` | systemctl 経由でサービス操作（デフォルト） |
| `exec` | 任意コマンドでサービス操作 |

**例: systemd バックエンド（ホスト環境）**

```yaml
agent:
  name: "web-server-01"
  mode: "standalone"
  backend: "systemd"
  tags:
    - "web"
    - "production"
```

**例: exec バックエンド（Docker コンテナ）**

```yaml
agent:
  name: "container-01"
  mode: "standalone"
  backend: "exec"
  tags:
    - "docker"
```

---

### 3.5 services - サービス定義（exec バックエンド用）

`agent.backend: exec` の場合に必要です。サービスごとに起動/停止/状態確認コマンドを定義します。

| キー | 型 | 必須 | 説明 |
|------|-----|------|------|
| `start` | string | Yes | サービス起動コマンド |
| `stop` | string | Yes | サービス停止コマンド |
| `status` | string | Yes | 状態確認コマンド（終了コード 0 = running） |
| `restart` | string | No | 再起動コマンド（未定義時は stop → start） |
| `working_dir` | string | No | 作業ディレクトリ |
| `env` | array[string] | No | 環境変数リスト（`KEY=VALUE` 形式） |

**例: 1コンテナ複数サービス**

```yaml
agent:
  backend: exec

services:
  nginx:
    start: "/usr/sbin/nginx"
    stop: "/usr/sbin/nginx -s quit"
    status: "pgrep -x nginx"
    
  redis:
    start: "/usr/bin/redis-server /etc/redis.conf --daemonize yes"
    stop: "/usr/bin/redis-cli shutdown"
    status: "/usr/bin/redis-cli ping"
    
  myapp:
    start: "/app/start.sh"
    stop: "/app/stop.sh"
    status: "/app/health.sh"
    working_dir: "/app"
    env:
      - "DATABASE_URL=postgres://localhost/mydb"
      - "REDIS_URL=redis://localhost:6379"
```

**例: 単一サービス**

```yaml
agent:
  backend: exec

services:
  app:
    start: "/entrypoint.sh"
    stop: "pkill -f entrypoint"
    status: "pgrep -f entrypoint"
```

---

### 3.6 retry - リトライ設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `max_attempts` | integer | `3` | 最大リトライ回数 |
| `delay_ms` | integer | `1000` | 初回リトライ遅延（ミリ秒） |
| `backoff_factor` | float | `2.0` | 指数バックオフ係数 |
| `max_delay_ms` | integer | `30000` | 最大リトライ遅延（ミリ秒） |

**リトライ遅延計算:**

```
delay = min(delay_ms * (backoff_factor ^ attempt), max_delay_ms)
```

**例: 攻撃的リトライ設定**

```yaml
retry:
  max_attempts: 5
  delay_ms: 500
  backoff_factor: 1.5
  max_delay_ms: 10000
```

---

### 3.7 timeout - タイムアウト設定

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `connect_seconds` | integer | `5` | TCP 接続タイムアウト |
| `read_seconds` | integer | `30` | HTTP レスポンス読み取りタイムアウト |
| `service_seconds` | integer | `60` | サービス起動/停止待機タイムアウト |

**例: 長時間起動サービス用**

```yaml
timeout:
  connect_seconds: 10
  read_seconds: 60
  service_seconds: 300  # 5分
```

---

### 3.8 acl - サービスアクセス制御（systemd バックエンド用）

`agent.backend: systemd` の場合に有効です。操作可能なサービスを制限します。

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `allowed` | array[string] | `[]` | 許可サービスリスト |
| `denied` | array[string] | `[]` | 拒否サービスリスト |

**評価順序:**

1. `denied` リストに一致 → 拒否
2. `allowed` リストが空 → 全許可
3. `allowed` リストに一致 → 許可
4. それ以外 → 拒否

**ワイルドカード対応:**

- `*` - 任意の文字列にマッチ
- `?` - 任意の1文字にマッチ

**例: 特定サービスのみ許可**

```yaml
acl:
  allowed:
    - "nginx"
    - "postgresql"
    - "redis"
    - "myapp-*"  # myapp- で始まるサービスすべて
  denied: []
```

**例: セキュリティ関連サービスを拒否**

```yaml
acl:
  allowed: []  # 全サービス許可
  denied:
    - "sshd"
    - "firewalld"
    - "systemd-*"
```

> **注意**: exec バックエンドでは `services` で定義されたサービスのみ操作可能なため、`acl` は不要です。

---

### 3.9 cluster - クラスタ設定（将来実装）

| キー | 型 | デフォルト | 説明 |
|------|-----|-----------|------|
| `enabled` | boolean | `false` | クラスタモード有効化 |
| `peers` | array[peer] | `[]` | ピアエージェント一覧 |

#### peer オブジェクト

| キー | 型 | 説明 |
|------|-----|------|
| `name` | string | ピア識別名 |
| `address` | string | アドレス（host:port） |

**例: クラスタ構成**

```yaml
cluster:
  enabled: true
  peers:
    - name: "agent-02"
      address: "192.168.1.102:8080"
    - name: "agent-03"
      address: "192.168.1.103:8080"
```

---

## 4. 環境変数

設定ファイルの値は環境変数で上書きできます。環境変数は設定ファイルより優先されます。

| 環境変数 | 対応設定 | 例 |
|----------|---------|-----|
| `SHIKI_CONFIG` | 設定ファイルパス | `/etc/shiki/config.yaml` |
| `SHIKI_SERVER_BIND` | `server.bind` | `0.0.0.0` |
| `SHIKI_SERVER_PORT` | `server.port` | `8080` |
| `SHIKI_AUTH_ENABLED` | `auth.enabled` | `true` |
| `SHIKI_AUTH_TOKEN` | `auth.token` | `your-secret-token` |
| `SHIKI_LOG_LEVEL` | `logging.level` | `debug` |
| `SHIKI_LOG_FORMAT` | `logging.format` | `json` |
| `SHIKI_AGENT_NAME` | `agent.name` | `web-server-01` |
| `SHIKI_AGENT_BACKEND` | `agent.backend` | `systemd` |

**例: Docker 環境での環境変数設定（systemd バックエンド）**

```bash
docker run -d \
  -e SHIKI_SERVER_PORT=8080 \
  -e SHIKI_AUTH_ENABLED=true \
  -e SHIKI_AUTH_TOKEN=secret \
  -e SHIKI_LOG_LEVEL=info \
  shiki:latest serve
```

**例: exec バックエンド（環境変数ではサービス定義不可、設定ファイル必須）**

```bash
docker run -d \
  -e SHIKI_AGENT_BACKEND=exec \
  -v ./config.yaml:/etc/shiki/config.yaml:ro \
  shiki:latest serve
```

---

## 5. 設定検証

### 5.1 設定ファイル検証

```bash
shiki config validate -c /etc/shiki/config.yaml
```

**出力例（成功時）:**

```
✓ Configuration is valid
  - Server: 0.0.0.0:8080
  - Auth: disabled
  - Logging: info (json)
  - Backend: systemd
  - ACL: 3 allowed, 2 denied
```

**出力例（exec バックエンド）:**

```
✓ Configuration is valid
  - Server: 0.0.0.0:8080
  - Auth: disabled
  - Logging: info (json)
  - Backend: exec
  - Services: nginx, redis, myapp
```

**出力例（エラー時）:**

```
✗ Configuration error at line 15:
  Invalid port number: 99999 (must be 1-65535)
```

### 5.2 現在の設定表示

```bash
shiki config show
```

**出力例:**

```yaml
server:
  bind: "0.0.0.0"
  port: 8080
  tls:
    enabled: false
# ... (以下省略)
```

### 5.3 設定の優先順位確認

```bash
shiki config show --sources
```

**出力例:**

```
server.port: 8080
  └─ source: environment variable (SHIKI_SERVER_PORT)

auth.token: ****
  └─ source: environment variable (SHIKI_AUTH_TOKEN)

logging.level: info
  └─ source: config file (/etc/shiki/config.yaml)
```

---

## 関連ドキュメント

- [DESIGN.md](DESIGN.md) - アーキテクチャ設計書
- [SPECIFICATION.md](SPECIFICATION.md) - 機能仕様書
- [API.md](API.md) - REST API リファレンス
- [SYSTEMD_INTEGRATION.md](SYSTEMD_INTEGRATION.md) - systemd 連携ガイド
- [examples/config.example.yaml](examples/config.example.yaml) - 設定ファイルサンプル
