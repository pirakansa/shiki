# API.md - shiki REST API リファレンス

> **Version**: 0.1.0  
> **Last Updated**: 2025-12-30  
> **Status**: Draft

---

## 1. 概要

### 1.1 ベース URL

```
http://{host}:{port}/api/v1
```

デフォルト: `http://localhost:8080/api/v1`

### 1.2 共通ヘッダー

#### リクエストヘッダー

| ヘッダー | 必須 | 説明 |
|----------|------|------|
| `Content-Type` | Yes（POST 時） | `application/json` |
| `Accept` | No | `application/json`（デフォルト） |
| `Authorization` | No | `Bearer {token}`（認証有効時） |
| `X-Request-ID` | No | リクエスト追跡用 UUID |

#### レスポンスヘッダー

| ヘッダー | 説明 |
|----------|------|
| `Content-Type` | `application/json` |
| `X-Request-ID` | リクエスト追跡用 UUID（リクエスト時に指定がなければ生成） |

### 1.3 共通レスポンス形式

#### 成功時

```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "timestamp": "2025-12-30T10:00:00Z"
}
```

#### 失敗時

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "E001",
    "message": "Human-readable error message",
    "details": { ... }
  },
  "timestamp": "2025-12-30T10:00:00Z"
}
```

---

## 2. エンドポイント一覧

| メソッド | パス | 説明 |
|----------|------|------|
| GET | `/health` | ヘルスチェック |
| GET | `/status` | エージェント状態取得 |
| POST | `/notify` | 通知受信・サービス操作実行 |
| GET | `/services` | サービス一覧取得 |
| GET | `/services/{name}` | サービス状態取得 |
| POST | `/services/{name}/start` | サービス起動 |
| POST | `/services/{name}/stop` | サービス停止 |
| POST | `/services/{name}/restart` | サービス再起動 |

---

## 3. エンドポイント詳細

### 3.1 GET /health

ヘルスチェック用エンドポイント。ロードバランサーや監視システムからの死活確認に使用。

#### リクエスト

```http
GET /api/v1/health HTTP/1.1
Host: localhost:8080
```

#### レスポンス（200 OK）

```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "version": "0.1.0",
    "uptime_seconds": 3600
  },
  "error": null,
  "timestamp": "2025-12-30T10:00:00Z"
}
```

#### レスポンスフィールド

| フィールド | 型 | 説明 |
|------------|-----|------|
| `status` | string | `healthy` / `degraded` / `unhealthy` |
| `version` | string | shiki バージョン |
| `uptime_seconds` | integer | 起動からの経過秒数 |

---

### 3.2 GET /status

エージェントの詳細な状態を取得。

#### リクエスト

```http
GET /api/v1/status HTTP/1.1
Host: localhost:8080
```

#### レスポンス（200 OK）

```json
{
  "success": true,
  "data": {
    "agent": {
      "name": "agent-01",
      "state": "ready",
      "mode": "standalone",
      "tags": ["web", "production"]
    },
    "server": {
      "bind": "0.0.0.0",
      "port": 8080,
      "tls_enabled": false
    },
    "stats": {
      "requests_total": 1234,
      "requests_success": 1200,
      "requests_failed": 34,
      "active_connections": 5
    },
    "version": "0.1.0",
    "uptime_seconds": 3600
  },
  "error": null,
  "timestamp": "2025-12-30T10:00:00Z"
}
```

---

### 3.3 POST /notify

他エージェントからの通知を受信し、指定されたサービス操作を実行する。

#### リクエスト

```http
POST /api/v1/notify HTTP/1.1
Host: localhost:8080
Content-Type: application/json

{
  "action": "start",
  "service": "nginx",
  "options": {
    "wait": true,
    "timeout_seconds": 60
  }
}
```

#### リクエストボディ

| フィールド | 型 | 必須 | 説明 |
|------------|-----|------|------|
| `action` | string | Yes | `start` / `stop` / `restart` |
| `service` | string | Yes | 対象サービス名（例: `nginx`） |
| `options` | object | No | オプション設定 |
| `options.wait` | boolean | No | 完了まで待機 [default: `true`] |
| `options.timeout_seconds` | integer | No | タイムアウト秒数 [default: `60`] |

#### レスポンス（200 OK）- wait: true

```json
{
  "success": true,
  "data": {
    "request_id": "550e8400-e29b-41d4-a716-446655440000",
    "service": "nginx",
    "action": "start",
    "result": "completed",
    "previous_status": "stopped",
    "current_status": "running",
    "duration_ms": 1234
  },
  "error": null,
  "timestamp": "2025-12-30T10:00:00Z"
}
```

#### レスポンス（202 Accepted）- wait: false

```json
{
  "success": true,
  "data": {
    "request_id": "550e8400-e29b-41d4-a716-446655440000",
    "service": "nginx",
    "action": "start",
    "result": "accepted",
    "message": "Request accepted, processing in background"
  },
  "error": null,
  "timestamp": "2025-12-30T10:00:00Z"
}
```

#### エラーレスポンス（404 Not Found）

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

#### エラーレスポンス（403 Forbidden）

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "E003",
    "message": "Service operation denied: sshd",
    "details": {
      "service": "sshd",
      "reason": "Service is in denied list"
    }
  },
  "timestamp": "2025-12-30T10:00:00Z"
}
```

#### エラーレスポンス（504 Gateway Timeout）

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "E005",
    "message": "Service operation timed out",
    "details": {
      "service": "nginx",
      "action": "start",
      "timeout_seconds": 60
    }
  },
  "timestamp": "2025-12-30T10:00:00Z"
}
```

---

### 3.4 GET /services

管理対象サービスの一覧を取得。

#### リクエスト

```http
GET /api/v1/services HTTP/1.1
Host: localhost:8080
```

#### クエリパラメータ

| パラメータ | 型 | 必須 | 説明 |
|------------|-----|------|------|
| `status` | string | No | 状態でフィルタ（`running` / `stopped` / `failed`） |
| `limit` | integer | No | 取得件数上限 [default: `100`] |
| `offset` | integer | No | オフセット [default: `0`] |

#### レスポンス（200 OK）

```json
{
  "success": true,
  "data": {
    "services": [
      {
        "name": "nginx",
        "status": "running",
        "enabled": true,
        "description": "A high performance web server"
      },
      {
        "name": "postgresql",
        "status": "stopped",
        "enabled": true,
        "description": "PostgreSQL database server"
      }
    ],
    "total": 2,
    "limit": 100,
    "offset": 0
  },
  "error": null,
  "timestamp": "2025-12-30T10:00:00Z"
}
```

---

### 3.5 GET /services/{name}

指定されたサービスの詳細な状態を取得。

#### リクエスト

```http
GET /api/v1/services/nginx HTTP/1.1
Host: localhost:8080
```

#### パスパラメータ

| パラメータ | 型 | 必須 | 説明 |
|------------|-----|------|------|
| `name` | string | Yes | サービス名 |

#### レスポンス（200 OK）

```json
{
  "success": true,
  "data": {
    "name": "nginx",
    "status": "running",
    "enabled": true,
    "description": "A high performance web server",
    "load_state": "loaded",
    "active_state": "active",
    "sub_state": "running",
    "main_pid": 12345,
    "started_at": "2025-12-30T09:00:00Z",
    "memory_current_bytes": 52428800,
    "tasks_current": 5
  },
  "error": null,
  "timestamp": "2025-12-30T10:00:00Z"
}
```

#### エラーレスポンス（404 Not Found）

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "E002",
    "message": "Service not found: unknown-service",
    "details": {
      "service": "unknown-service"
    }
  },
  "timestamp": "2025-12-30T10:00:00Z"
}
```

---

### 3.6 POST /services/{name}/start

指定されたサービスを起動。

#### リクエスト

```http
POST /api/v1/services/nginx/start HTTP/1.1
Host: localhost:8080
Content-Type: application/json

{
  "wait": true,
  "timeout_seconds": 30
}
```

#### リクエストボディ

| フィールド | 型 | 必須 | 説明 |
|------------|-----|------|------|
| `wait` | boolean | No | 完了まで待機 [default: `true`] |
| `timeout_seconds` | integer | No | タイムアウト秒数 [default: `60`] |

#### レスポンス（200 OK）

```json
{
  "success": true,
  "data": {
    "service": "nginx",
    "action": "start",
    "previous_status": "stopped",
    "current_status": "running",
    "duration_ms": 523
  },
  "error": null,
  "timestamp": "2025-12-30T10:00:00Z"
}
```

---

### 3.7 POST /services/{name}/stop

指定されたサービスを停止。

#### リクエスト

```http
POST /api/v1/services/nginx/stop HTTP/1.1
Host: localhost:8080
Content-Type: application/json

{
  "wait": true,
  "timeout_seconds": 30
}
```

#### レスポンス（200 OK）

```json
{
  "success": true,
  "data": {
    "service": "nginx",
    "action": "stop",
    "previous_status": "running",
    "current_status": "stopped",
    "duration_ms": 234
  },
  "error": null,
  "timestamp": "2025-12-30T10:00:00Z"
}
```

---

### 3.8 POST /services/{name}/restart

指定されたサービスを再起動。

#### リクエスト

```http
POST /api/v1/services/nginx/restart HTTP/1.1
Host: localhost:8080
Content-Type: application/json

{
  "wait": true,
  "timeout_seconds": 60
}
```

#### レスポンス（200 OK）

```json
{
  "success": true,
  "data": {
    "service": "nginx",
    "action": "restart",
    "previous_status": "running",
    "current_status": "running",
    "duration_ms": 1523
  },
  "error": null,
  "timestamp": "2025-12-30T10:00:00Z"
}
```

---

## 4. エラーコード一覧

| HTTP Status | Error Code | 説明 |
|-------------|------------|------|
| 400 | E008 | リクエストが不正（パラメータ不足、形式エラー等） |
| 401 | E007 | 認証失敗（トークン不正、期限切れ等） |
| 403 | E003 | サービス操作が許可されていない |
| 404 | E002 | サービスが見つからない |
| 500 | E004 | systemd 操作エラー |
| 502 | E006 | 接続エラー |
| 503 | E009 | エージェントがビジー状態 |
| 504 | E005 | タイムアウト |

---

## 5. 認証（オプション）

認証が有効な場合、すべてのエンドポイント（`/health` を除く）で `Authorization` ヘッダーが必要です。

### 5.1 Bearer トークン認証

```http
GET /api/v1/status HTTP/1.1
Host: localhost:8080
Authorization: Bearer your-secret-token
```

### 5.2 認証エラー

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "E007",
    "message": "Authentication failed",
    "details": {
      "reason": "Invalid or expired token"
    }
  },
  "timestamp": "2025-12-30T10:00:00Z"
}
```

---

## 6. レート制限（将来実装）

| 制限 | 値 | 説明 |
|------|-----|------|
| リクエスト/分 | 100 | IP アドレスごと |
| バースト | 20 | 瞬間最大リクエスト数 |

レート制限超過時は `429 Too Many Requests` を返却。

---

## 関連ドキュメント

- [DESIGN.md](DESIGN.md) - アーキテクチャ設計書
- [SPECIFICATION.md](SPECIFICATION.md) - 機能仕様書
- [CONFIGURATION.md](CONFIGURATION.md) - 設定リファレンス
- [SYSTEMD_INTEGRATION.md](SYSTEMD_INTEGRATION.md) - systemd 連携ガイド
