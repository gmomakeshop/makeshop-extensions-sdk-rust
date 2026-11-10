//! Input and output types and helpers for the `order_completed` event.
//!
//! Delivery may be retried, so event handlers must be idempotent.

#[doc(hidden)]
pub mod bindings {
    wit_bindgen::generate!({
        path: "wit/order-completed/v1",
        world: "order-completed-guest",
        generate_all,
        additional_derives: [serde::Serialize, serde::Deserialize],
        pub_export_macro: true,
        export_macro_name: "export_order_completed",
        default_bindings_module: "makeshop_extensions::order_completed::bindings",
    });
}

pub use bindings::exports::makeshop::order_completed::handler::{Guest, Input, OrderCompletedData};
pub use bindings::makeshop::event_types::types::{DeliveryContext, DeliveryFailure, DeliveryResult, EventMetadata};
pub use bindings::makeshop::order_snapshot::types::{
    AppliedCoupon,
    Buyer,
    Order,
    OrderedProduct,
    OrderedProductVariation,
    ProductKind,
    Receiver,
};
pub use bindings::makeshop::runtime_types::common::{GuestError, ShopMetafieldEntry};

pub use crate::host::{connector, debug, error, execution, info, uuid, warn};

/// Reservation of a mail to the buyer of the delivered order.
///
/// The runtime does not send the mail. It hands the reservation to the platform together with a
/// successful delivery result, and the platform sends it to the buyer's address on record after
/// the delivery is committed. A retry, a dead-letter, a guest error, or a runtime failure discards
/// every reservation of the run, so a handler that reserves a mail and then returns
/// [`DeliveryResult::Retry`] sends nothing until an attempt succeeds. A delivery succeeds at
/// most once, so its mails are sent at most once.
///
/// The mail is sent regardless of whether the buyer allows the shop's mail magazine. Check
/// `order.buyer.mail_magazine_allowed` before reserving a mail with promotional content.
///
/// The body is free-form text or HTML. The runtime checks only that it is not empty, fits the
/// size limit, and contains no control characters other than tab, line feed, and carriage
/// return; it does not validate or sanitize markup.
///
/// When sending, the platform replaces `[MAIL_HEAD]` in the body with the shop's common mail
/// header and `[MAIL_TAIL]` with its common mail footer, inserted as the shop set them. The
/// rest of the body is sent as it is, and the subject is never expanded.
pub mod mail {
    pub use super::bindings::makeshop::mail::buyer::{BuyerMail, MailBodyFormat, MailError, MailErrorCode};

    impl BuyerMail {
        /// Builds a mail sent as plain text.
        ///
        /// `operation_key` names the mail within the delivery, such as `"order-thanks"`, and
        /// identifies the reservation in the run log. It is 1 to 128 ASCII characters: letters,
        /// digits, `.`, `_`, `:`, and `-`, and must be unique within one run.
        pub fn text(operation_key: impl Into<String>, subject: impl Into<String>, body: impl Into<String>) -> Self {
            Self {
                operation_key: operation_key.into(),
                subject: subject.into(),
                format: MailBodyFormat::Text,
                body: body.into(),
            }
        }

        /// Builds a mail sent as an HTML document. The runtime does not validate or sanitize the
        /// markup.
        pub fn html(operation_key: impl Into<String>, subject: impl Into<String>, body: impl Into<String>) -> Self {
            Self {
                operation_key: operation_key.into(),
                subject: subject.into(),
                format: MailBodyFormat::Html,
                body: body.into(),
            }
        }
    }

    /// Reserves a mail to the buyer of the delivered order.
    ///
    /// The runtime validates the operation key, the subject, and the body before accepting the
    /// reservation, so a rejected mail never reaches the buyer in a partial form. The number
    /// of reservations per run is limited.
    ///
    /// # Errors
    ///
    /// Returns a [`MailError`] whose [`MailErrorCode`] tells which part was rejected: a malformed
    /// or duplicate operation key; a subject that is empty or whitespace only, longer than 200
    /// characters, or contains a control character or an invisible formatting character such as
    /// a bidirectional control, zero width space, or byte order mark; a body that is empty or
    /// whitespace only, too large, or contains a control character other than tab, line feed,
    /// and carriage return; or a run that has reached its reservation limit.
    pub fn send_to_buyer(mail: &BuyerMail) -> Result<(), MailError> {
        #[cfg(target_arch = "wasm32")]
        {
            super::bindings::makeshop::mail::buyer::send_to_buyer(mail)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::testing::dispatch_buyer_mail(mail.clone())
        }
    }
}

/// Returns a result that completes delivery.
pub fn ok() -> Result<DeliveryResult, GuestError> {
    Ok(DeliveryResult::Ok)
}

/// Returns a result that requests a retry after a temporary failure.
pub fn retry(failure: DeliveryFailure) -> Result<DeliveryResult, GuestError> {
    Ok(DeliveryResult::Retry(failure))
}

/// Returns a result that sends the event to dead letter after a permanent failure.
pub fn dead_letter(failure: DeliveryFailure) -> Result<DeliveryResult, GuestError> {
    Ok(DeliveryResult::DeadLetter(failure))
}

/// Test input fixtures.
///
/// [`fixture::input`] returns an [`Input`] populated with valid values.
#[cfg(not(target_arch = "wasm32"))]
pub mod fixture {
    use super::{Buyer, DeliveryContext, EventMetadata, Input, Order, OrderCompletedData, OrderedProduct, Receiver};

    /// Returns an [`Input`] for testing.
    pub fn input() -> Input {
        Input {
            context: DeliveryContext {
                shop_id: "shop-1".to_string(),
                subscription_id: "sub-1".to_string(),
                delivery_id: "delivery-1".to_string(),
                delivery_attempt: 1,
            },
            event: EventMetadata {
                event_id: "event-1".to_string(),
                event_type: "order_completed".to_string(),
                event_schema_version: 1,
                source: "makeshop".to_string(),
                subject: "shop-1".to_string(),
                occurred_at: "2026-08-06T00:00:00+09:00".to_string(),
                published_at: "2026-08-06T00:00:01+09:00".to_string(),
                sequence_number: Some(1),
                correlation_id: Some("018f0000-0000-7000-8000-000000000001".to_string()),
                causation_id: None,
                replay_id: None,
            },
            shop_metafields: Vec::new(),
            data: OrderCompletedData {
                order: order(),
                applied_coupon: None,
            },
        }
    }

    /// Returns an [`Order`] for testing.
    pub fn order() -> Order {
        Order {
            display_order_number: "20260806-000001".to_string(),
            member_id: Some("member-1".to_string()),
            buyer: Buyer {
                name: "試験 太郎".to_string(),
                name_kana: "シケン タロウ".to_string(),
                zip_code: "1500001".to_string(),
                prefecture: "東京都".to_string(),
                address1: "渋谷区1-1-1".to_string(),
                address2: None,
                tel: "0300000000".to_string(),
                email: "taro@example.com".to_string(),
                company_name: None,
                company_name_kana: None,
                department_name: None,
                mail_magazine_allowed: Some(true),
            },
            receivers: vec![Receiver {
                name: "試験 花子".to_string(),
                name_kana: "シケン ハナコ".to_string(),
                zip_code: "1500002".to_string(),
                prefecture: "東京都".to_string(),
                address1: "渋谷区2-2-2".to_string(),
                address2: None,
                tel: "0300000001".to_string(),
                company_name: None,
                company_name_kana: None,
                department_name: None,
            }],
            shop_point_discount: 0,
            total_price: 1000,
            payment_method_code: "credit".to_string(),
            products: vec![OrderedProduct {
                name: "テスト商品".to_string(),
                system_code: "000000000001".to_string(),
                custom_code: None,
                jan_code: None,
                kind: None,
                variation: None,
                quantity: 1,
            }],
        }
    }
}
