# makeshop-extensions-sdk-macros

makeshop by GMO の Extensions で、guest の entrypoint を宣言する attribute macro。

**この crate を直接依存に書く必要はありません。**
[makeshop-extensions-sdk](https://crates.io/crates/makeshop-extensions-sdk) が
`#[makeshop_extensions::hook(...)]` / `#[makeshop_extensions::event(...)]` として再 export
するので、そちらを使ってください。使い方も SDK 側の README にあります。

proc-macro crate は macro 以外の item を export できないという Rust の制約があるため、SDK 本体
とは別 crate に分かれています。依存ツリーでこの名前を見かけるのはそのためです。

## License

Licensed under the Apache License, Version 2.0.
See the bundled `LICENSE` file.
