---
title: "AWS EC2を使ってリージョン制限を突破するCLIツールをRustで作った"
emoji: "🌍"
type: "tech"
topics: ["rust", "aws", "cli", "proxy", "opensource"]
published: false
---

# はじめに

「日本からアクセスできない」「このコンテンツはお住まいの地域では利用できません」

こんなメッセージを見たことはありませんか？リージョン制限は、ストリーミングサービス、ゲーム、APIなど、さまざまな場面で私たちの前に立ちはだかります。

今回、この問題を解決するために **region-proxy** というCLIツールをRustで作りました。

https://github.com/M-Igashi/region-proxy

## デモ

実際の動作をご覧ください。コマンド一発で東京リージョンにプロキシを立ち上げ、IPアドレスが日本に変わるのを確認し、停止時に全リソースが自動削除されます。

![region-proxy demo](https://raw.githubusercontent.com/M-Igashi/region-proxy/master/docs/demo.gif)

## region-proxy とは

region-proxyは、**AWS EC2インスタンスを任意のリージョンで起動し、SSHダイナミックポートフォワーディングでSOCKS5プロキシを構築する**CLIツールです。

```bash
# 東京リージョンでプロキシを起動
$ region-proxy start --region ap-northeast-1

# これだけで、ローカルの localhost:1080 が東京経由のSOCKSプロキシになる
```

### 特徴

- 🌍 **全AWSリージョン対応**: 33のリージョンから選択可能
- 🚀 **ワンコマンド**: 約30秒でプロキシが使える
- 💰 **激安**: t4g.nano使用で約$0.004/時間（月$3程度）
- 🧹 **自動クリーンアップ**: 停止時にAWSリソースを全削除
- 🍎 **macOS統合**: システム全体のプロキシを自動設定

## なぜ作ったのか

### 既存ソリューションの課題

| 方法 | 課題 |
|------|------|
| 商用VPN | 月額$5-15、リージョンが限定的 |
| 手動EC2 + SSH | セットアップが面倒、クリーンアップを忘れがち |
| AWS SSM | 設定が複雑、IAMポリシーが大変 |

「AWSアカウントさえあれば、コマンド一発で好きなリージョンのプロキシを立てたい」

この単純な願望から region-proxy は生まれました。

## 他のソリューションとの比較

| 機能 | region-proxy | 手動EC2 | VPNサービス |
|------|-------------|---------|------------|
| セットアップ時間 | 約30秒 | 約10分 | 様々 |
| コスト | ~$0.004/時間 | 同じ | $5-15/月 |
| AWSリージョン | 全33箇所 | 全33箇所 | 限定的 |
| 自動クリーンアップ | ✅ | ❌ | N/A |
| サブスク不要 | ✅ | ✅ | ❌ |
| オープンソース | ✅ | N/A | ❌ |

## アーキテクチャ

```
┌─────────────┐     SSH Tunnel      ┌──────────────┐     ┌──────────────┐
│   Your Mac  │ ──────────────────► │  EC2 (Tokyo) │ ──► │   Internet   │
│  SOCKS:1080 │    Port Forwarding  │   t4g.nano   │     │  (JP region) │
└─────────────┘                     └──────────────┘     └──────────────┘
```

### 動作フロー

1. **EC2起動**: 指定リージョンで最小インスタンス（t4g.nano）を起動
2. **セキュリティグループ**: あなたのIPからのSSHのみ許可
3. **キーペア生成**: セッション用の一時SSHキーを作成
4. **SSHトンネル**: `-D`オプションでダイナミックポートフォワーディング
5. **システム設定**: macOSのネットワーク設定を自動更新

## 技術的なこだわり

### Rustを選んだ理由

- **シングルバイナリ**: 依存関係なしで配布可能
- **クロスコンパイル**: ARM/x86両対応のユニバーサルバイナリ
- **型安全性**: AWSリージョンの扱いミスを防止
- **非同期処理**: tokioによる効率的なI/O

### AWS SDK for Rust

```rust
use aws_sdk_ec2::{Client, types::Filter};

async fn find_latest_ami(client: &Client, region: &str) -> Result<String> {
    let resp = client
        .describe_images()
        .owners("amazon")
        .filters(
            Filter::builder()
                .name("name")
                .values("al2023-ami-*-kernel-*-arm64")
                .build(),
        )
        .send()
        .await?;
    
    // 最新のAMIを取得
    // ...
}
```

AWS SDK for Rustはまだ比較的新しいですが、十分に実用的でした。

### エラーハンドリング

`anyhow` + `thiserror` の組み合わせで、ユーザーフレンドリーなエラーメッセージを実現：

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("No AWS credentials found. Run 'aws configure' first.")]
    NoCredentials,
    
    #[error("Region '{0}' is not supported")]
    UnsupportedRegion(String),
    
    #[error("EC2 instance failed to start: {0}")]
    InstanceStartFailed(String),
}
```

### 状態管理

実行中のプロキシ情報は `~/.region-proxy/state.json` に保存：

```json
{
  "instance_id": "i-0abc123def456789",
  "region": "ap-northeast-1",
  "public_ip": "54.168.xxx.xxx",
  "ssh_pid": 12345,
  "started_at": "2024-01-15T10:30:00Z"
}
```

これにより、クラッシュ後の復旧や孤立リソースのクリーンアップが可能に。

## 使い方

### インストール

```bash
# Homebrew（推奨）
brew tap M-Igashi/tap
brew install region-proxy

# または cargo
cargo install --git https://github.com/M-Igashi/region-proxy
```

### 基本操作

```bash
# デフォルトリージョンを設定
region-proxy config set-region ap-northeast-1

# プロキシ開始
region-proxy start

# 状態確認
region-proxy status

# 停止（リソース自動削除）
region-proxy stop
```

### 利用可能なリージョン一覧

```bash
$ region-proxy list-regions

Available AWS Regions:

  ap-northeast-1 (Tokyo)
  ap-northeast-2 (Seoul)
  ap-southeast-1 (Singapore)
  us-east-1 (N. Virginia)
  us-west-2 (Oregon)
  eu-west-1 (Ireland)
  eu-central-1 (Frankfurt)
  ...
```

## セキュリティ

- 🔑 SSHキーはセッションごとに生成され、自動削除される
- 🛡️ セキュリティグループはあなたのIPアドレスのみ許可
- 💾 EC2インスタンスは停止時に終了（永続データなし）
- 🏠 認証情報はローカルに保持（AWS以外に送信されない）

## コスト

### 実コスト

| インスタンス | 時間単価 | 1日8時間 | 月間（24/7） |
|-------------|---------|---------|-------------|
| t4g.nano | $0.0042 | $0.034 | $3.02 |
| t3.nano | $0.0052 | $0.042 | $3.74 |

**使った分だけ**の課金なので、必要なときだけ起動すれば月額数十円〜数百円程度です。

## ユースケース

- 🎮 **ゲーム**: リージョン制限のあるゲームサーバーやコンテンツにアクセス
- 📺 **ストリーミング**: 地域制限のある動画コンテンツを視聴
- 🧪 **テスト**: 異なる地理的位置からアプリケーションをテスト
- 🔒 **プライバシー**: 異なるリージョン経由でトラフィックをルーティング
- 💼 **開発**: リージョン固有のAPIやサービスにアクセス

## 今後の予定

- [ ] Linux対応
- [ ] 複数同時接続
- [ ] 接続時間制限
- [ ] コスト表示機能
- [ ] IPv6対応

## まとめ

region-proxy は、AWSを使って手軽にリージョン制限を突破するためのツールです。

- VPNサービスに課金したくない
- 特定のリージョンからのアクセスが必要
- AWSアカウントは持っている

こんな方にぴったりです。ぜひ試してみてください！

https://github.com/M-Igashi/region-proxy

⭐ スターをいただけると励みになります！

## 参考リンク

- [GitHub リポジトリ](https://github.com/M-Igashi/region-proxy)
- [AWS SDK for Rust](https://aws.amazon.com/sdk-for-rust/)
- [SSH ダイナミックポートフォワーディング](https://www.ssh.com/academy/ssh/tunneling-example)
