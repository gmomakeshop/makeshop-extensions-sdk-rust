//! Input and output types and helpers for the `finalize_checkout_validation` hook.
//!
//! This hook determines whether checkout may be finalized.

#[doc(hidden)]
pub mod bindings {
    wit_bindgen::generate!({
        path: "wit/finalize-checkout-validation/v1",
        world: "finalize-checkout-validation-guest",
        generate_all,
        additional_derives: [serde::Serialize, serde::Deserialize],
        pub_export_macro: true,
        export_macro_name: "export_finalize_checkout_validation",
        default_bindings_module: "makeshop_extensions::finalize_checkout_validation::bindings",
    });
}

pub use bindings::exports::makeshop::finalize_checkout_validation::handler::{
    AppliedCoupon,
    Buyer,
    Checkout,
    CheckoutProduct,
    CheckoutProductVariation,
    Decision,
    Guest,
    GuestError,
    Input,
    PlannedOrder,
    ProductKind,
    Receiver,
    ShopMetafieldEntry,
};

pub use crate::host::{connector, debug, error, execution, info, uuid, warn};

/// Returns a decision that allows checkout to be finalized.
pub fn pass() -> Result<Decision, GuestError> {
    Ok(Decision::Pass)
}

/// Returns a decision that prevents checkout from being finalized.
///
/// Provide `messages` in display order for the buyer. Record operational details in a runtime
/// log with [`info`], [`warn`], or [`error`].
pub fn fail(messages: Vec<String>) -> Result<Decision, GuestError> {
    Ok(Decision::Fail(messages))
}

/// Returns a decision that prevents checkout from being finalized and displays the default message.
pub fn fail_with_default_message() -> Result<Decision, GuestError> {
    Ok(Decision::Fail(Vec::new()))
}

/// Test input fixtures.
///
/// [`fixture::input`] returns an [`Input`] populated with valid values.
#[cfg(not(target_arch = "wasm32"))]
pub mod fixture {
    use super::{Buyer, Checkout, CheckoutProduct, Input, PlannedOrder, Receiver};

    /// Returns an [`Input`] for testing.
    pub fn input() -> Input {
        Input {
            shop_id: "shop-1".to_string(),
            checkout_session_uuid: "018f0000-0000-7000-8000-000000000001".to_string(),
            viewer_user_agent: "test-agent/1.0".to_string(),
            viewer_ip_address: "203.0.113.1".to_string(),
            checkout: Checkout {
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
                planned_orders: vec![PlannedOrder {
                    products: vec![CheckoutProduct {
                        name: "テスト商品".to_string(),
                        system_code: "000000000001".to_string(),
                        custom_code: None,
                        jan_code: None,
                        kind: None,
                        variation: None,
                        quantity: 1,
                    }],
                }],
                applied_coupon: None,
                shop_point_discount: 0,
                total_price: 1000,
                payment_method_code: "credit".to_string(),
            },
            shop_metafields: Vec::new(),
        }
    }
}
