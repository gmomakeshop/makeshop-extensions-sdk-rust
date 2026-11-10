//! Input and output types for the `order_completed_content` hook.
//!
//! This hook returns content for the order completion page. Use [`crate::content_blocks`] to build
//! the content.

#[doc(hidden)]
pub mod bindings {
    wit_bindgen::generate!({
        path: "wit/order-completed-content/v1",
        world: "order-completed-content-guest",
        generate_all,
        additional_derives: [serde::Serialize, serde::Deserialize],
        pub_export_macro: true,
        export_macro_name: "export_order_completed_content",
        default_bindings_module: "makeshop_extensions::order_completed_content::bindings",
    });
}

pub use bindings::exports::makeshop::order_completed_content::handler::{
    ContentEntry,
    ContentFragment,
    Guest,
    Input,
    Output,
};
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

/// Test input fixtures.
///
/// [`fixture::input`] and [`fixture::order`] return values populated with valid data.
#[cfg(not(target_arch = "wasm32"))]
pub mod fixture {
    use super::{Buyer, Input, Order, OrderedProduct, Receiver};

    /// Returns an [`Input`] for testing.
    pub fn input() -> Input {
        Input {
            shop_id: "shop-1".to_string(),
            event_id: "event-1".to_string(),
            checkout_session_uuid: "018f0000-0000-7000-8000-000000000001".to_string(),
            applied_coupon: None,
            viewer_user_agent: "test-agent/1.0".to_string(),
            viewer_ip_address: "203.0.113.1".to_string(),
            orders: vec![order()],
            shop_metafields: Vec::new(),
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
