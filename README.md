<h1 align="center">makeshop Extensions SDK for Rust</h1>

<p align="center">
  <strong>makeshopの処理を拡張するWebAssemblyコンポーネントをRustで開発できます。</strong><br />
  型付きのentrypoint契約、host API、content block builderを提供します。
</p>

<p align="center">
  SDKs: <a href="https://github.com/gmomakeshop/makeshop-extensions-sdk-go">Go</a> · <a href="https://github.com/gmomakeshop/makeshop-extensions-sdk-rust">Rust</a>
</p>

---

## 目次

- [このSDKについて](#このsdkについて)
- [導入](#導入)
- [最小実装](#最小実装)
- [DeploymentConfig](#deploymentconfig)
- [SDKの構成](#sdkの構成)

---

## このSDKについて

makeshop Extensionsは、makeshopの処理に独自のロジックを追加するための仕組みです。
本SDKを使った拡張機能の利用には、対応プランの契約が必要です。

Rust SDKは次のentrypointを公開しています。

| Entrypoint | 種別 | Cargo feature | 用途 |
|---|---|---|---|
| `finalize_checkout_validation` | Hook | `finalize-checkout-validation-v1` | 注文確定前の検証 |
| `order_completed_content` | Hook | `order-completed-content-v1` | 注文完了画面へのcontent追加 |
| `order_completed` | Event | `order-completed-v1` | 注文完了後の処理 |

一つのcrateでは、一つのentrypoint featureだけを有効にします。
Rust 1.99以降と`wasm32-wasip2` targetが必要です。

---

## 導入

拡張機能のcrateで`cdylib`を指定し、実装するentrypointのfeatureを有効にします。

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
makeshop-extensions-sdk = { version = "0.2", features = ["finalize-checkout-validation-v1"] }
```

Handlerを実装したら、WebAssemblyコンポーネントをビルドします。

```console
rustup target add wasm32-wasip2
cargo build --release --target wasm32-wasip2
```

---

## 最小実装

### Hook

次の例は、合計金額が100,000円を超える注文を停止します。

```rust
use makeshop_extensions::finalize_checkout_validation::{
    Decision, GuestError, Input, fail, pass,
};

#[makeshop_extensions::hook(finalize_checkout_validation)]
fn run(input: Input) -> Result<Decision, GuestError> {
    if input.checkout.total_price > 100_000 {
        return fail(vec!["注文金額が上限を超えています。".to_string()]);
    }

    pass()
}
```

`Decision::Fail`のmessageは購入者に表示される可能性があります。
運用情報はmessageに含めず、runtime logへ出力してください。

### Event

次の例は、注文完了Eventを受け取り、処理済みとして返します。

```rust
use makeshop_extensions::order_completed::{GuestError, Input, DeliveryResult, info, ok};

#[makeshop_extensions::event(order_completed)]
fn deliver(input: Input) -> Result<DeliveryResult, GuestError> {
    info(
        &format!("order completed: {}", input.event.event_id),
        (),
    );
    ok()
}
```

外部サービスを呼び出す場合は`connector::call`を使用します。
一時的な失敗は`retry`、再試行しても解消しない失敗は`dead_letter`として返します。

---

## DeploymentConfig

WebAssemblyコンポーネントと`DeploymentConfig`は一つのartifactとして登録します。
EntrypointはWIT契約から判定されるため、JSONには指定しません。
`metadata.name`は表示用の名前であり、HookやEventの指定ではありません。

次の例は、`order_webhook`というconnectorを定義します。

```json
{
  "metadata": {
    "name": "order-export"
  },
  "runtime": {
    "timeout_ms": 3000
  },
  "connectors": [
    {
      "name": "order_webhook",
      "method": "POST",
      "endpoint": "https://orders.example.com/events",
      "required": false,
      "timeout_ms": 2000,
      "request": {
        "headers": {
          "Content-Type": "application/json"
        }
      }
    }
  ]
}
```

Handlerから呼び出す`ConnectorRequest.connector`には、`connectors[].name`と同じ値を指定します。
Connectorを使わない場合は`connectors: []`とします。
本番環境のendpointにはHTTPSを使用してください。

---

## SDKの構成

| Crate | Version |
|---|---|
| [`makeshop-extensions-content-blocks`](https://crates.io/crates/makeshop-extensions-content-blocks) | 0.1.1 |
| [`makeshop-extensions-sdk-macros`](https://crates.io/crates/makeshop-extensions-sdk-macros) | 0.1.0 |
| [`makeshop-extensions-sdk`](https://crates.io/crates/makeshop-extensions-sdk) | 0.2.1 |

各crateのREADMEは、crates.ioにも掲載されます。

---

## リポジトリについて

このリポジトリは配布用のスナップショットです。
IssueとPull Requestは受け付けていません。
Releaseはcrates.ioへ公開されます。

## ライセンス

[LICENSE](LICENSE)を参照してください。
