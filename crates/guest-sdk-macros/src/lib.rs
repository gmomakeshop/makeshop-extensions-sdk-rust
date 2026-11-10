//! Attribute macros for declaring makeshop extension entrypoints.
//!
//! Most users should use [`hook`] and [`event`] through their re-exports from
//! `makeshop-extensions-sdk`.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Error, Ident, ItemFn, parse_macro_input};

// Entrypoint category.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Hook,
    Event,
}

impl Kind {
    // Name of the corresponding attribute macro.
    fn macro_name(self) -> &'static str {
        match self {
            Self::Hook => "hook",
            Self::Event => "event",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Hook => "Hook",
            Self::Event => "Event",
        }
    }
}

// Information required to generate an entrypoint.
struct EntrypointSpec {
    kind: Kind,
    module: &'static str,
    method: &'static str,
    output: &'static str,
    export_macro: &'static str,
}

// Derive module and export macro names from an entrypoint name.
macro_rules! spec {
    ($kind:ident, $name:literal, $method:literal, $output:literal) => {
        (
            $name,
            EntrypointSpec {
                kind: Kind::$kind,
                module: $name,
                method: $method,
                output: $output,
                export_macro: concat!("export_", $name),
            },
        )
    };
}

const ENTRYPOINT_SPECS: &[(&str, EntrypointSpec)] = &[
    spec!(Hook, "finalize_checkout_validation", "run", "Decision"),
    spec!(Event, "order_completed", "deliver", "DeliveryResult"),
    spec!(Hook, "order_completed_content", "run", "Output"),
];

/// Declares a hook entrypoint.
///
/// ```ignore
/// #[makeshop_extensions::hook(finalize_checkout_validation)]
/// fn run(input: Input) -> Result<Decision, GuestError> { ... }
/// ```
///
/// Pass the hook name as the argument and enable the Cargo feature with the same name.
/// Use [`macro@event`] for event entrypoints.
#[proc_macro_attribute]
pub fn hook(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_attribute(Kind::Hook, attr, item)
}

/// Declares an event entrypoint.
///
/// ```ignore
/// #[makeshop_extensions::event(order_completed)]
/// fn deliver(input: Input) -> Result<DeliveryResult, GuestError> { ... }
/// ```
///
/// `DeliveryResult::Ok` completes delivery, `DeliveryResult::Retry` requests another attempt, and
/// `DeliveryResult::DeadLetter` stops delivery without retrying. Because delivery may be retried,
/// handlers must be idempotent. Use [`macro@hook`] for hook entrypoints.
///
/// [`DeliveryResult`]: https://docs.rs/makeshop-extensions-sdk
#[proc_macro_attribute]
pub fn event(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_attribute(Kind::Event, attr, item)
}

fn expand_attribute(kind: Kind, attr: TokenStream, item: TokenStream) -> TokenStream {
    let name = parse_macro_input!(attr as Ident);
    let entrypoint = parse_macro_input!(item as ItemFn);
    expand(kind, &name, entrypoint)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand(used: Kind, name: &Ident, entrypoint: ItemFn) -> Result<proc_macro2::TokenStream, Error> {
    let name_str = name.to_string();
    let Some((_, spec)) = ENTRYPOINT_SPECS
        .iter()
        .find(|(candidate, _)| *candidate == name_str)
    else {
        let known: Vec<&str> = ENTRYPOINT_SPECS
            .iter()
            .filter(|(_, spec)| spec.kind == used)
            .map(|(candidate, _)| *candidate)
            .collect();
        return Err(Error::new(name.span(), format!("unknown {} `{name_str}`; supported: {known:?}", used.label())));
    };
    // Confusing hooks and events invalidates retry and idempotency assumptions; direct users to the
    // correct macro.
    if spec.kind != used {
        return Err(Error::new(
            name.span(),
            format!(
                "`{name_str}` is {}, so use `#[makeshop_extensions::{}({name_str})]` instead of `{}`",
                spec.kind.label(),
                spec.kind.macro_name(),
                used.macro_name(),
            ),
        ));
    }
    if let Some(asyncness) = entrypoint.sig.asyncness {
        return Err(Error::new_spanned(
            asyncness,
            "an entrypoint must be a synchronous fn, because the WIT contract declares a sync func; \
             call connectors in parallel with makeshop_extensions::host::connector::call_batch",
        ));
    }
    if entrypoint.sig.inputs.len() != 1 {
        return Err(Error::new_spanned(&entrypoint.sig, "an entrypoint takes exactly one input argument"));
    }

    let fn_name = entrypoint.sig.ident.clone();
    let module = Ident::new(spec.module, Span::call_site());
    let method = Ident::new(spec.method, Span::call_site());
    let output = Ident::new(spec.output, Span::call_site());
    let export_macro = Ident::new(spec.export_macro, Span::call_site());
    let entrypoint_mod = quote!(::makeshop_extensions::#module);
    // Component wiring is wasm32-only. Native builds retain the entrypoint as an ordinary function
    // so cargo tests can call it through the makeshop_extensions::testing harness.
    Ok(quote! {
        #[cfg_attr(
            not(target_arch = "wasm32"),
            allow(dead_code, reason = "The entrypoint is exported only on wasm32; native tests may call it directly.")
        )]
        #entrypoint

        #[cfg(target_arch = "wasm32")]
        #[doc(hidden)]
        mod __makeshop_extensions_entrypoint_export {
            struct Component;

            impl #entrypoint_mod::bindings::exports::makeshop::#module::handler::Guest for Component {
                fn #method(
                    input: #entrypoint_mod::Input,
                ) -> ::core::result::Result<#entrypoint_mod::#output, #entrypoint_mod::GuestError> {
                    // Record the panic cause in the tenant's runtime log before trapping.
                    ::makeshop_extensions::__internal::init_panic_hook();
                    super::#fn_name(input)
                }
            }

            ::makeshop_extensions::#export_macro!(Component);
        }
    })
}
