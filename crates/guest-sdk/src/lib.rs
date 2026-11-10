//! SDK for implementing makeshop guest extensions in Rust.
//!
//! A guest implements exactly one entrypoint. Enable the Cargo feature for that entrypoint and
//! annotate its function with [`hook`] or [`event`]; the attribute generates the component
//! wiring. Enabling several entrypoint features at once fails to build, because their host
//! imports would be mixed into one component.
//!
//! The package is named `makeshop-extensions-sdk` and is imported as `makeshop_extensions`.
//!
//! # Hooks and events
//!
//! A hook interrupts the surrounding operation: it runs synchronously, and the value it returns
//! decides what happens next. An event is a delivery: a failed handler is retried, and a handler
//! that gives up sends the delivery to dead-letter, so an event handler must be idempotent.
//! Annotating an entrypoint with the wrong attribute is a compile error that names the other one.
//!
//! # Contract versions
//!
//! The `-vN` suffix of an entrypoint feature is the major version of its contract, which moves
//! independently of the crate version. A contract is not rewritten in place: a new major arrives
//! as an additional feature, so migrating changes one line in `Cargo.toml`. There is no
//! unsuffixed alias meaning "the latest contract", so a rebuild alone never changes the contract
//! a guest is built against.
//!
//! # Host APIs and tests
//!
//! [`host`] provides connector calls, execution context, UUID generation, and runtime logging as
//! canonical SDK types rather than per-entrypoint bindings, so guest code reads the same across
//! entrypoints. Endpoints, credentials, retries, and default headers come from the deployment
//! configuration; a guest names a connector and builds a body.
//!
//! The attributes expand to component wiring only for `wasm32` targets, so in a native build an
//! entrypoint stays an ordinary function that `cargo test` can call. [`testing`] provides the
//! harness that supplies connector responses, an execution context, and log capture.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub use makeshop_extensions_macros::{event, hook};

pub mod host;

/// Internal wiring used only by `#[makeshop_extensions::hook]` expansion.
/// Guest code must not call this module directly.
#[doc(hidden)]
pub mod __internal {
    use std::sync::Once;

    static PANIC_HOOK: Once = Once::new();

    // Installs a hook once to record panic details in the runtime log.
    pub fn init_panic_hook() {
        PANIC_HOOK.call_once(|| {
            std::panic::set_hook(Box::new(|panic_info| {
                let message = panic_info
                    .payload()
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_string())
                    .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "non-string panic payload".to_string());
                let location = panic_info
                    .location()
                    .map(|location| format!("{}:{}", location.file(), location.line()))
                    .unwrap_or_else(|| "unknown".to_string());
                crate::host::error(
                    "guest panic",
                    serde_json::json!({
                        "panic_message": message,
                        "location": location,
                    }),
                );
            }));
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod testing;

#[cfg(feature = "order-completed-content-v1")]
pub use content_blocks;

#[cfg(feature = "finalize-checkout-validation-v1")]
pub mod finalize_checkout_validation;
#[cfg(feature = "order-completed-v1")]
pub mod order_completed;
#[cfg(feature = "order-completed-content-v1")]
pub mod order_completed_content;

#[cfg(feature = "finalize-checkout-validation-v1")]
#[doc(hidden)]
pub use finalize_checkout_validation::bindings::export_finalize_checkout_validation;
#[cfg(feature = "order-completed-v1")]
#[doc(hidden)]
pub use order_completed::bindings::export_order_completed;
#[cfg(feature = "order-completed-content-v1")]
#[doc(hidden)]
pub use order_completed_content::bindings::export_order_completed_content;
