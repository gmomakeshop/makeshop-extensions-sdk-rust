# makeshop-extensions-content-blocks

makeshop by GMO の Extensions で表示コンテンツを組み立てるための Block 語彙（serde 型 + builder + 検証）。

`order_completed_content` のような content hook が返すのは描画そのものではなく **意味** です。
見出し・強調・キー値ペア・注意枠といった構造だけを返し、色・余白・フォントは店舗テーマ側の
renderer が決めます。この crate が扱うのはその「何を伝えるか」の語彙で、v1 は 16 type
（leaf 12 種 + コンテナ 4 種）を持ちます。

## 位置づけ

ふつうはこの crate を直接依存に書く必要はありません。content hook 向けの feature を有効にした
`makeshop-extensions-sdk` が `makeshop_extensions::content_blocks` として再 export します。

```toml
[dependencies]
makeshop-extensions-sdk = { version = "0.2", features = ["order-completed-content-v1"] }
```

Block を組んで検証するだけの用途（別プロセスでの事前検査など）では単体でも使えます。

```toml
[dependencies]
makeshop-extensions-content-blocks = "0.1"
```

パッケージ名と lib 名は別で、`use` するパスは `content_blocks` です。
`wasm32-unknown-unknown` を含む環境でそのままコンパイルできるよう、std のネットワーク機能や
外部 URL パーサには依存していません。

## 組み立てる

block ごとの型を `blocks!` で並べ、`blocks_to_json` で 1 fragment 分の `blocks-json` にします。

```rust
use content_blocks::{Alert, Block, Card, Copyable, Grid, Section, Text, Variant, blocks, blocks_to_json};

let page: Vec<Block> = blocks![
    Alert::new(Variant::Success, blocks![
        Text::plain("クーポンを配布しました"),
        Copyable::new("CPN-9999").label("クーポンコード"),
    ]),
    Section::new(blocks![Grid::new(blocks![Card::new("おすすめ A"), Card::new("おすすめ B")]).columns_hint(2)])
        .heading("特集"),
];
let blocks_json = blocks_to_json(&page);
assert!(blocks_json.starts_with(r#"[{"type":"alert""#));
```

`blocks_to_json` は失敗しません。子スロットに置ける型は親の型が持っているため、階層に反する
組み合わせは**コンパイルエラー**になります。

- `section` の子 — `grid` / `alert` / `accordion` / leaf（`section` の入れ子は不可）
- `grid` の item — `card` / `image` のみ
- `alert` / `accordion` の子 — leaf のみ
- `table` の列数と各行のセル数

そのため builder は `Result` を返しません。JSON から読み込む経路（`parse_blocks`）では、同じ
違反がパースエラーになります（未知の type・未知フィールドも同様に拒否されます）。

## 手元で検証する

ホストは受け取った出力を厳格に検証し、違反があれば fail-open（表示なし）でその実行を失敗に
します。**一部だけ表示される挙動はありません**。同じ検査をこの crate が公開しているので、
デプロイ前に `cargo test` で落とせます。

```rust
use content_blocks::{FragmentInput, validate_fragments};

let fragments = [FragmentInput {
    placement: "main",
    blocks_json: r#"[{"type":"link","text":"詳細","url":"https://example.com/promo"}]"#,
}];
validate_fragments(&fragments, &["main"]).expect("valid output");
```

`placement` は挿入位置の ID で、hook ごとに決まった許可一覧からしか選べません
（`order_completed_content` v1 は `main` の 1 点のみ）。

## 制限

| 項目 | 上限 | 定数 |
| --- | --- | --- |
| `fragments` 件数（1 出力あたり） | 8 | `MAX_FRAGMENTS` |
| 全 fragment 合計の block 数 | 64 | `MAX_TOTAL_BLOCKS` |
| `blocks-json` サイズ（1 fragment） | 8192 バイト | `MAX_FRAGMENT_BLOCKS_JSON_BYTES` |
| text 系フィールド | 1024 バイト | `MAX_TEXT_BYTES` |
| `fields.entries` 件数 | 16 | `MAX_FIELDS_ENTRIES` |
| `table.columns` 件数 | 6 | `MAX_TABLE_COLUMNS` |
| `table.rows` 件数 | 20 | `MAX_TABLE_ROWS` |
| `list.items` 件数 | 16 | `MAX_LIST_ITEMS` |
| `grid.items` 件数 | 12 | `MAX_GRID_ITEMS` |
| `badge.text` | 64 バイト | `MAX_BADGE_TEXT_BYTES` |
| `qr.value` | 512 バイト | `MAX_QR_VALUE_BYTES` |
| `heading.level` | 1〜3 | `HEADING_LEVEL` |
| `grid.columns-hint` | 2〜4 | `GRID_COLUMNS_HINT` |

件数系の下限は常に 1 で、空は許されません。サイズはすべて UTF-8 のバイト数で、文字数では
ありません。値は定数として公開しているので、guest 側から同じ数字を参照できます。

```rust
use content_blocks::*;

assert_eq!(SCHEMA_VERSION, 1);
assert_eq!((MAX_FRAGMENTS, MAX_TOTAL_BLOCKS, MAX_FRAGMENT_BLOCKS_JSON_BYTES), (8, 64, 8192));
assert_eq!((MAX_TEXT_BYTES, MAX_FIELDS_ENTRIES), (1024, 16));
assert_eq!((MAX_TABLE_COLUMNS, MAX_TABLE_ROWS, MAX_LIST_ITEMS, MAX_GRID_ITEMS), (6, 20, 16, 12));
assert_eq!((MAX_BADGE_TEXT_BYTES, MAX_QR_VALUE_BYTES), (64, 512));
assert_eq!((HEADING_LEVEL, GRID_COLUMNS_HINT), ((1, 3), (2, 4)));
```

この一覧に無い上限（`data` 件数・出力合計サイズなど）は、この crate ではなくホスト側が
検査します。

## URL 規則

`link.url` / `image.url` は次をすべて満たす必要があります。**接続先そのものの許可リストは
無い**ので、条件を満たす限り任意の https URL を返せます。

1. `https://` で始まる
2. 制御文字・空白・バックスラッシュを含まない
3. authority に userinfo（`@`）を含まない
4. host が空でない

`qr.value` はこの規則の対象外です（renderer は画像化するだけで URL として辿りません）。

## License

Licensed under the Apache License, Version 2.0.
See the bundled `LICENSE` file.
