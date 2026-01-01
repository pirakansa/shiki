# DESIGN.md - shiki アーキテクチャ設計書

> **Version**: 0.2.0  
> **Last Updated**: 2025-12-30  
> **Status**: Draft

---

## 1. 概要

### 1.1 目的

shiki は、**複数マシン間・複数コンテナ間でのサービス起動順序を HTTP ベースで連携させる**ための軽量ツールです。

systemd 単体では単一マシン内の依存関係しか制御できませんが、shiki を導入することで、ネットワーク越しに「Machine A のサービスが起動したら Machine B のサービスを起動する」といった連携が可能になります。

また、systemd が動作しない Docker コンテナ環境でも、任意のコマンドを実行するバックエンドにより、同様のサービス起動順序制御が可能です。

### 1.2 設計原則

| 原則 | 説明 |
|------|------|
| **シンプルさ** | REST API と YAML 設定のみで動作。複雑なオーケストレーションは行わない |
| **軽量性** | 静的リンクされた単一バイナリ。メモリ使用量は最小限 |
| **汎用性** | ホスト環境でも Docker コンテナ内でも動作可能 |
| **非侵襲性** | 既存の systemd ユニットファイルを変更せず、ExecStartPre/Post で連携 |
| **可観測性** | 構造化ログと状態 API により、動作状況を把握しやすい |
| **拡張性** | バックエンド抽象化により、systemd 以外の環境にも対応 |

### 1.3 用語定義

| 用語 | 定義 |
|------|------|
| **Agent** | 各マシン/コンテナ上で動作する shiki の常駐プロセス。HTTP サーバーとして起動指示を受け付ける |
| **Backend** | サービス操作の実行方法。`systemd`（systemctl）または `exec`（任意コマンド） |
| **Notify** | あるエージェントから別のエージェントへサービス起動/停止を依頼する HTTP リクエスト |
| **Target** | 通知先のエージェントのアドレス（host:port） |
| **Service** | 管理対象のサービス。systemd ユニットまたは exec で定義されたコマンドセット |

---

## 2. システムアーキテクチャ

### 2.1 全体構成図

```mermaid
graph TB
    subgraph "Machine A (Web Server)"
        UA[nginx.service]
        AA[shiki Agent :8080]
    end
    
    subgraph "Machine B (Database Server)"
        UB[postgresql.service]
        AB[shiki Agent :8080]
    end
    
    subgraph "Docker Container C"
        UC[myapp process]
        AC[shiki Agent :8080]
    end
    
    AA -->|"POST /notify"| AB
    AA -->|"POST /notify"| AC
    AB -->|"systemctl start"| UB
    AC -->|"exec: /app/start.sh"| UC
    
    UA -.->|"ExecStartPre"| AA
```

### 2.2 バックエンドアーキテクチャ

```mermaid
graph LR
    subgraph "shiki Agent"
        HTTP[HTTP Server]
        CTRL[Service Controller]
        
        subgraph "Backend"
            SYSD[systemd Backend]
            EXEC[exec Backend]
        end
    end
    
    HTTP --> CTRL
    CTRL --> SYSD
    CTRL --> EXEC
    
    SYSD -->|systemctl| SD[systemd]
    EXEC -->|spawn| CMD[Commands]
```

### 2.3 デプロイメントパターン

#### パターン A: ホスト環境（systemd バックエンド）

```
┌─────────────────────────────────┐
│           Host OS               │
│  ┌─────────┐    ┌────────────┐  │
│  │ shiki   │    │ systemd    │  │
│  │ agent   │───▶│ services   │  │
│  │(systemd)│    └────────────┘  │
│  └─────────┘                    │
└─────────────────────────────────┘
```

#### パターン B: Docker コンテナ内（exec バックエンド）

```
┌─────────────────────────────────┐
│       Docker Container          │
│  ┌─────────┐    ┌────────────┐  │
│  │ shiki   │    │ User       │  │
│  │ agent   │───▶│ Processes  │  │
│  │ (exec)  │    └────────────┘  │
│  └─────────┘                    │
└─────────────────────────────────┘
```

#### パターン C: 1コンテナ複数サービス（exec バックエンド）

```
┌─────────────────────────────────────┐
│         Docker Container            │
│  ┌─────────┐                        │
│  │ shiki   │    ┌────────────┐      │
│  │ agent   │───▶│ nginx      │      │
│  │ (exec)  │    └────────────┘      │
│  │         │    ┌────────────┐      │
│  │         │───▶│ redis      │      │
│  │         │    └────────────┘      │
│  │         │    ┌────────────┐      │
│  │         │───▶│ myapp      │      │
│  └─────────┘    └────────────┘      │
└─────────────────────────────────────┘
```

### 2.4 通信フロー

```mermaid
sequenceDiagram
    participant App as Application Service
    participant Agent1 as shiki Agent (Machine A)
    participant Agent2 as shiki Agent (Machine B)
    participant Backend as Backend (systemd/exec)
    
    App->>Agent1: ExecStartPre: shiki notify
    Agent1->>Agent2: POST /api/v1/notify
    Agent2->>Backend: start service
    Backend-->>Agent2: Service started
    Agent2-->>Agent1: 200 OK {status: "started"}
    Agent1-->>App: Exit 0 (success)
    Note over App: Application starts
```

---

## 3. バックエンド設計

### 3.1 バックエンド一覧

| バックエンド | 説明 | 用途 |
|--------------|------|------|
| `systemd` | systemctl 経由でサービス操作 | ホスト環境 |
| `exec` | 任意コマンドでサービス操作 | Docker コンテナ、systemd 非対応環境 |

### 3.2 systemd バックエンド

systemctl コマンドを使用してサービスを操作します。

```yaml
agent:
  backend: systemd
```

| アクション | 実行コマンド |
|------------|-------------|
| `start` | `systemctl start <service>` |
| `stop` | `systemctl stop <service>` |
| `restart` | `systemctl restart <service>` |
| `status` | `systemctl is-active <service>` |

### 3.3 exec バックエンド

任意のコマンドを実行してサービスを操作します。サービスごとにコマンドを定義します。

```yaml
agent:
  backend: exec
  services:
    nginx:
      start: "/usr/sbin/nginx"
      stop: "/usr/sbin/nginx -s quit"
      status: "pgrep -x nginx"
    redis:
      start: "/usr/bin/redis-server --daemonize yes"
      stop: "/usr/bin/redis-cli shutdown"
      status: "/usr/bin/redis-cli ping"
```

**特徴:**
- 1コンテナ内で複数サービスを管理可能
- systemd の代替として Docker 環境で利用
- カスタムスクリプトによる柔軟な制御

---

## 4. コンポーネント設計

### 4.1 コンポーネント構成図

```mermaid
graph LR
    subgraph "shiki Agent"
        CLI[CLI Parser]
        CFG[Config Loader]
        HTTP[HTTP Server]
        NOTIFY[Notify Handler]
        SVC[Service Controller]
        LOG[Logger]
        
        subgraph "Backends"
            SYSD[systemd]
            EXEC[exec]
        end
    end
    
    CLI --> CFG
    CFG --> HTTP
    HTTP --> NOTIFY
    HTTP --> SVC
    NOTIFY --> SVC
    SVC --> SYSD
    SVC --> EXEC
    SVC --> LOG
```

### 4.2 主要コンポーネント

| コンポーネント | 責務 | 主な依存 crate |
|----------------|------|----------------|
| **CLI Parser** | コマンドライン引数の解析 | `clap` |
| **Config Loader** | YAML 設定ファイルの読み込み・検証 | `serde_yaml` |
| **HTTP Server** | REST API エンドポイントの提供 | `axum` |
| **Notify Handler** | 他エージェントへの通知送信 | `reqwest` |
| **Service Controller** | バックエンド経由でサービス操作 | - |
| **systemd Backend** | systemctl コマンド実行 | `std::process::Command` |
| **exec Backend** | 任意コマンド実行 | `std::process::Command` |
| **Logger** | 構造化ログ出力 | `tracing` |

### 4.3 モジュール構成（予定）

```
src/
├── main.rs              # エントリーポイント
├── lib.rs               # ライブラリルート
├── cli.rs               # CLI 定義
├── config.rs            # 設定構造体・ローダー
├── server/
│   ├── mod.rs           # HTTP サーバー
│   ├── routes.rs        # ルーティング定義
│   └── handlers.rs      # リクエストハンドラ
├── notify.rs            # 通知送信ロジック
├── service/
│   ├── mod.rs           # Service Controller
│   ├── backend.rs       # Backend トレイト定義
│   ├── systemd.rs       # systemd バックエンド
│   └── exec.rs          # exec バックエンド
└── error.rs             # エラー型定義
```

---

## 5. 技術選定

| 領域 | 選定技術 | 選定理由 |
|------|---------|----------|
| 言語 | Rust | 軽量バイナリ、メモリ安全性、クロスコンパイル容易 |
| HTTP Server | axum | 軽量、async 対応、Tower エコシステム |
| HTTP Client | reqwest | 使いやすい API、async 対応 |
| Config Parser | serde_yaml | serde エコシステム、YAML 標準対応 |
| CLI Parser | clap | 豊富な機能、derive マクロ対応 |
| Logging | tracing | 構造化ログ、async 対応 |
| Error Handling | thiserror | 軽量なエラー型定義 |
| Async Runtime | tokio | デファクトスタンダード |

---

## 6. 今後の拡張ポイント

以下は初期リリース後に検討する機能です：

- [ ] **TLS 対応**: HTTPS 通信の暗号化
- [ ] **認証機能**: Bearer トークン / mTLS による認証
- [ ] **クラスタモード**: 複数エージェント間の自動検出・連携
- [ ] **Web UI**: 状態確認用のダッシュボード
- [ ] **永続化**: 状態の SQLite 保存

---

## 関連ドキュメント

- [SPECIFICATION.md](SPECIFICATION.md) - 機能仕様書
- [API.md](API.md) - REST API リファレンス
- [CONFIGURATION.md](CONFIGURATION.md) - 設定リファレンス
- [SYSTEMD_INTEGRATION.md](SYSTEMD_INTEGRATION.md) - systemd 連携ガイド
