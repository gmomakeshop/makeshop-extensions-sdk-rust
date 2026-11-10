# makeshop-extensions-sdk

makeshop by GMO の Extensions を WebAssembly component として書くための Rust SDK。

Extensions は、ショップの処理の途中にテナント自身のロジックを差し込む仕組みです。この SDK は
wit-bindgen の定型（`generate!` 呼び出し・export 配線・ログラッパ）を引き受けるので、guest crate は
**属性を付けた関数を 1 つ書くだけ**で component になります。

```rust
use makeshop_extensions::finalize_checkout_validation::{Decision, GuestError, Input, info};

#[makeshop_extensions::hook(finalize_checkout_validation)]
fn run(input: Input) -> Result<Decision, GuestError> {
    info("received", serde_json::json!({ "shop_id": &input.shop_id }));
    Ok(Decision::Pass)
}
```

パッケージ名は `makeshop-extensions-sdk` ですが、`use` するパスは `makeshop_extensions` です。

導入手順・ビルド・`DeploymentConfig` の書き方は
[リポジトリの README](https://github.com/gmomakeshop/makeshop-extensions-sdk-rust#readme)、
API の詳細は [docs.rs](https://docs.rs/makeshop-extensions-sdk) にあります。

## entrypoint と feature

**1 guest = 1 entrypoint = 1 feature** です。複数を同時に有効化することはできません
（import surface が混ざるため、wasm ビルド時に `compile_error!` で止まります）。実装する
entrypoint に応じて、属性は `#[hook]` と `#[event]` を使い分けます。

**Hook** は処理を同期で割り込み、返した判定がそのまま次の処理を左右します。**Event** は配送で
あり、失敗すればリトライされ、諦めれば dead-letter に落ちます。つまり Event のハンドラは
**冪等でなければなりません**。取り違えたときは、正しい方へ誘導するコンパイルエラーになります。

feature 名の末尾 `-v1` は**契約のバージョン**で、crate のバージョンとは別軸です。契約は
書き換えず、新しい major を feature として足す形で増えるので、移行は Cargo.toml の 1 行を
差し替える作業になります。無印を「最新版のエイリアス」にはしていません（Cargo.toml を
変えずに再ビルドしただけで契約が変わる事態を避けるため）。

## host API

外部との通信・実行コンテキスト・UUID・ログは `makeshop_extensions::host` から使います。
entrypoint ごとに生成される bindings の型ではなく SDK 正準型を公開しているので、entrypoint を
跨いでも同じ書き方になります。

接続先・認証・リトライ・デフォルトヘッダは deploy 時の設定で管理し、guest 側は connector 名と
body の組み立てに集中します。JSON API には型付きヘルパ `connector::call_json` があり、
request の直列化・2xx 判定・応答の逆直列化を引き受けて、失敗を `CallJsonError` の variant
（`Encode` / `Connector` / `UnexpectedStatus` / `Decode`）へ分類します。

汎用 request は `ConnectorRequest::builder` で構築できます。
connector 名、timeout hint、追加 header は `build` で検証され、直接 struct を構築した場合も送信前に同じ検証が走ります。

表示コンテンツを返す entrypoint の feature を有効にすると、Block 語彙が
`makeshop_extensions::content_blocks` として使えます
（[makeshop-extensions-content-blocks](https://crates.io/crates/makeshop-extensions-content-blocks)
を SDK 経由で配布しているので、直接依存に書く必要はありません）。

`order_completed` では `makeshop_extensions::order_completed::mail` から購入者宛メールを予約できます。
`BuyerMail::text` または `BuyerMail::html` で件名と本文から組み立て、`mail::send_to_buyer` に渡します。
本文は自由文で、platform は整形も無害化もしません。
本文中の `[MAIL_HEAD]` と `[MAIL_TAIL]` だけは、送信時にショップの共通ヘッダーと共通フッターへ置き換わります（件名は置き換えません）。
送信は platform が行い、handler が `Ok` を返したあとに注文の購入者へ届きます（`Retry` / `DeadLetter`
では予約は捨てられます）。配送が `Ok` で完了するのは 1 回だけなので、同じメールが二度送られることはありません。
`operation_key` は run 内でメールを識別する名前で、ASCII 英数字と `.` `_` `:` `-` からなる 128 文字以内の値にします。
メールはメルマガ受信可否にかかわらず送られます。販促を含むメールは `order.buyer.mail_magazine_allowed`
（会員購入でのみ値があります）を確認してから予約してください。
テストでは `TestHost::buyer_mails` で予約内容を読めます。

## deploy せずにテストする

属性が component 配線を生成するのは wasm32 ビルドのときだけなので、native ビルドでは
entrypoint は素の関数のままです。`cargo test` からそのまま呼べます。

```rust
#[cfg(test)]
mod tests {
    use makeshop_extensions::finalize_checkout_validation::{Decision, fixture};
    use makeshop_extensions::testing::{TestHost, json_response};

    #[test]
    fn 在庫apiが拒否したら注文確定を止める() {
        let host = TestHost::new()
            .with_connector("stock-api", |_req| json_response(200, r#"{"approved":false}"#));
        let decision = host.run(|| super::run(fixture::input()));
        assert!(matches!(decision, Ok(Decision::Fail(_))));
    }
}
```

- `TestHost` が connector 応答・実行コンテキスト・ログ捕捉をモックします。内蔵の executor で
  駆動するので tokio 等は要りません。
- 各 entrypoint の `fixture::input()` が全フィールド埋め済みの入力を返すので、テストは必要な
  フィールドだけ書き換えます。
- 未登録 connector の呼び出しや、ハーネス外からの host 呼び出しは、握り潰さずに明確な
  メッセージで panic します。
- 本番で guest が panic した場合も、メッセージと発生位置がテナント自身の runtime log に
  `guest panic` として残ってから実行が失敗します。

## License

Licensed under the Apache License, Version 2.0.
See the bundled `LICENSE` file.
