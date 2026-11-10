//! APIs for unit-testing guest extensions.
//!
//! Configure connector responses and an execution context on [`TestHost`], then run the entrypoint.
//!
//! ```ignore
//! let host = TestHost::new()
//!     .with_connector("stock-api", |_req| testing::json_response(200, r#"{"approved":true}"#));
//! let decision = host.run(run(fixture::input()));
//! assert!(matches!(decision, Ok(Decision::Pass)));
//! assert!(host.logs().iter().any(|log| log.message == "received"));
//! ```

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::host::{
    ConnectorError,
    ConnectorRequest,
    ConnectorResponse,
    ExecutionContext,
    LogLevel,
    LogRecord,
    PayloadFormat,
};

#[cfg(feature = "order-completed-v1")]
use crate::order_completed::mail::{BuyerMail, MailError, MailErrorCode};

type ConnectorHandler = Rc<dyn Fn(ConnectorRequest) -> Result<ConnectorResponse, ConnectorError>>;

/// Number of buyer mails one run may reserve. The runtime enforces the same limit.
#[cfg(feature = "order-completed-v1")]
const MAX_BUYER_MAILS_PER_RUN: usize = 3;

/// Size limit of a buyer mail body in bytes. The runtime enforces the same limit.
#[cfg(feature = "order-completed-v1")]
const MAX_BUYER_MAIL_BODY_BYTES: usize = 128 * 1024;

/// Length limit of a buyer mail subject in characters. The runtime enforces the same limit.
#[cfg(feature = "order-completed-v1")]
const MAX_BUYER_MAIL_SUBJECT_CHARS: usize = 200;

/// Length limit of a buyer mail operation key in bytes, as fixed by the contract.
#[cfg(feature = "order-completed-v1")]
const MAX_OPERATION_KEY_BYTES: usize = 128;

/// Reports whether `key` is a well-formed operation key: 1 to 128 ASCII letters, digits,
/// `.`, `_`, `:`, and `-`.
#[cfg(feature = "order-completed-v1")]
fn is_valid_operation_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_OPERATION_KEY_BYTES
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

/// Checks a buyer mail subject against the runtime rules and returns the rejection message.
///
/// The subject must contain a non-whitespace character, be at most
/// [`MAX_BUYER_MAIL_SUBJECT_CHARS`] characters, and contain no undisplayable character.
#[cfg(feature = "order-completed-v1")]
fn check_buyer_mail_subject(subject: &str) -> Result<(), String> {
    if subject.trim().is_empty() {
        return Err("subject must not be empty".to_string());
    }
    let chars = subject.chars().count();
    if chars > MAX_BUYER_MAIL_SUBJECT_CHARS {
        return Err(format!("subject exceeds {MAX_BUYER_MAIL_SUBJECT_CHARS} characters (got {chars})"));
    }
    if subject.chars().any(is_undisplayable) {
        return Err("subject must not contain control or formatting characters".to_string());
    }
    Ok(())
}

/// Reports whether `ch` is rejected in text shown to the buyer.
///
/// Besides control characters, this covers formatting characters that can disguise the display
/// order or appearance: zero width space, left-to-right and right-to-left marks, byte order mark,
/// and bidirectional embeddings, overrides, and isolates. Zero width joiner (U+200D) is allowed
/// because emoji sequences need it.
#[cfg(feature = "order-completed-v1")]
fn is_undisplayable(ch: char) -> bool {
    ch.is_control()
        || matches!(ch, '\u{200B}' | '\u{200E}' | '\u{200F}' | '\u{FEFF}')
        || ('\u{202A}'..='\u{202E}').contains(&ch)
        || ('\u{2066}'..='\u{2069}').contains(&ch)
}

thread_local! {
    static ACTIVE: RefCell<Option<ActiveHarness>> = const { RefCell::new(None) };
    static UUID_COUNTER: Cell<u64> = const { Cell::new(0) };
}

struct ActiveHarness {
    connectors: HashMap<String, ConnectorHandler>,
    context: ExecutionContext,
    logs: Rc<RefCell<Vec<LogRecord>>>,
    #[cfg(feature = "order-completed-v1")]
    buyer_mails: Rc<RefCell<Vec<BuyerMail>>>,
}

/// Host implementation for unit-testing guest extensions.
///
/// Configure connector responses and the execution context before calling [`TestHost::run`].
pub struct TestHost {
    connectors: HashMap<String, ConnectorHandler>,
    context: ExecutionContext,
    logs: Rc<RefCell<Vec<LogRecord>>>,
    #[cfg(feature = "order-completed-v1")]
    buyer_mails: Rc<RefCell<Vec<BuyerMail>>>,
}

/// Returns the current time used by the default execution context of the test harness.
fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

impl Default for TestHost {
    fn default() -> Self {
        Self::new()
    }
}

impl TestHost {
    /// Returns a `TestHost` with no registered connectors.
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
            // Anchor the context to the current time. With a fixed time in the past
            // `deadline_unix_ms` would always leave zero remaining, and the APIs that read the
            // remaining time (`connector::call_with_retry` and the like) would take a different
            // branch in tests than in production. A test that asserts on the time itself passes a
            // fixed context through `with_execution_context`.
            context: ExecutionContext {
                run_id: "test-run".to_string(),
                started_at_unix_ms: now_unix_ms(),
                deadline_unix_ms: now_unix_ms() + 5_000,
            },
            logs: Rc::new(RefCell::new(Vec::new())),
            #[cfg(feature = "order-completed-v1")]
            buyer_mails: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Registers a test handler for the named connector.
    ///
    /// The test panics if the guest calls an unregistered connector.
    pub fn with_connector(
        mut self,
        name: impl Into<String>,
        handler: impl Fn(ConnectorRequest) -> Result<ConnectorResponse, ConnectorError> + 'static,
    ) -> Self {
        self.connectors.insert(name.into(), Rc::new(handler));
        self
    }

    /// Sets the execution context used by the test.
    pub fn with_execution_context(mut self, context: ExecutionContext) -> Self {
        self.context = context;
        self
    }

    /// Activates this `TestHost` and executes `body`.
    ///
    /// Panics if `run` calls are nested.
    pub fn run<T>(&self, body: impl FnOnce() -> T) -> T {
        struct Uninstall;
        impl Drop for Uninstall {
            fn drop(&mut self) {
                ACTIVE.with(|active| active.borrow_mut().take());
            }
        }

        ACTIVE.with(|active| {
            let mut slot = active.borrow_mut();
            assert!(slot.is_none(), "TestHost::run must not be nested");
            *slot = Some(ActiveHarness {
                connectors: self.connectors.clone(),
                context: self.context.clone(),
                logs: Rc::clone(&self.logs),
                #[cfg(feature = "order-completed-v1")]
                buyer_mails: Rc::clone(&self.buyer_mails),
            });
        });
        // Reset the UUID sequence for each run so test execution order cannot affect results.
        UUID_COUNTER.with(|counter| counter.set(0));
        let _uninstall = Uninstall;
        body()
    }

    /// Returns runtime logs captured by previous [`TestHost::run`] calls.
    pub fn logs(&self) -> Vec<LogRecord> {
        self.logs.borrow().clone()
    }

    /// Returns buyer mails reserved by previous [`TestHost::run`] calls, in reservation order.
    ///
    /// The harness keeps every accepted reservation regardless of the delivery result the handler
    /// returned. The platform sends them only after a successful delivery.
    #[cfg(feature = "order-completed-v1")]
    pub fn buyer_mails(&self) -> Vec<BuyerMail> {
        self.buyer_mails.borrow().clone()
    }
}

/// Returns a successful connector response containing a JSON body.
pub fn json_response(status: u16, body: &str) -> Result<ConnectorResponse, ConnectorError> {
    Ok(ConnectorResponse {
        status,
        format: PayloadFormat::Json,
        body: body.as_bytes().to_vec(),
        headers: Vec::new(),
        upstream_trace_id: None,
    })
}

pub(crate) fn dispatch_connector(req: ConnectorRequest) -> Result<ConnectorResponse, ConnectorError> {
    ACTIVE.with(|active| {
        let active = active.borrow();
        let Some(harness) = active.as_ref() else {
            panic!(
                "connector::call runs only inside TestHost::run; install makeshop_extensions::testing::TestHost first"
            );
        };
        let Some(handler) = harness.connectors.get(&req.connector) else {
            let registered: Vec<&str> =
                harness.connectors.keys().map(String::as_str).collect();
            panic!(
                "connector \"{}\" is not registered on the TestHost (registered: {registered:?}); define its response with with_connector",
                req.connector
            );
        };
        handler(req)
    })
}

pub(crate) fn dispatch_context() -> ExecutionContext {
    ACTIVE.with(|active| {
        active.borrow().as_ref().map(|harness| harness.context.clone()).unwrap_or_else(|| {
            panic!(
                "execution::get_context runs only inside TestHost::run; install makeshop_extensions::testing::TestHost first"
            )
        })
    })
}

pub(crate) fn dispatch_uuid(version: u8) -> String {
    // Preserve the UUID version field while using a sequence to keep test results deterministic.
    let count = UUID_COUNTER.with(|counter| {
        let next = counter.get() + 1;
        counter.set(next);
        next
    });
    format!("00000000-0000-{version}000-8000-{count:012x}")
}

/// Applies the same acceptance rules as the runtime, so a reservation that fails in production also
/// fails in the test. Markup is not inspected, as in production.
#[cfg(feature = "order-completed-v1")]
pub(crate) fn dispatch_buyer_mail(mail: BuyerMail) -> Result<(), MailError> {
    fn reject(code: MailErrorCode, message: &str) -> MailError {
        MailError {
            code,
            message: message.to_string(),
        }
    }

    ACTIVE.with(|active| {
        let active = active.borrow();
        let Some(harness) = active.as_ref() else {
            panic!(
                "mail::send_to_buyer runs only inside TestHost::run; install makeshop_extensions::testing::TestHost first"
            );
        };
        let mut reserved = harness.buyer_mails.borrow_mut();
        if reserved.len() >= MAX_BUYER_MAILS_PER_RUN {
            return Err(reject(MailErrorCode::LimitExceeded, "buyer mail limit exceeded"));
        }
        if !is_valid_operation_key(&mail.operation_key) {
            return Err(reject(
                MailErrorCode::OperationKeyInvalid,
                "operation-key must be 1 to 128 ASCII letters, digits, '.', '_', ':', and '-'",
            ));
        }
        if reserved.iter().any(|existing| existing.operation_key == mail.operation_key) {
            return Err(reject(
                MailErrorCode::OperationKeyInvalid,
                "operation-key is already reserved in this run",
            ));
        }
        if let Err(message) = check_buyer_mail_subject(&mail.subject) {
            return Err(reject(MailErrorCode::SubjectInvalid, &message));
        }
        if mail.body.trim().is_empty() {
            return Err(reject(MailErrorCode::BodyInvalid, "body must not be empty"));
        }
        if mail.body.len() > MAX_BUYER_MAIL_BODY_BYTES {
            return Err(reject(MailErrorCode::BodyInvalid, "body exceeds the size limit"));
        }
        if mail
            .body
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\t' | '\n' | '\r'))
        {
            return Err(reject(
                MailErrorCode::BodyInvalid,
                "body must not contain control characters other than tab, line feed, and carriage return",
            ));
        }
        reserved.push(mail);
        Ok(())
    })
}

pub(crate) fn dispatch_log(level: LogLevel, message: &str, attrs_json: &str) {
    let record = LogRecord {
        level,
        message: message.to_string(),
        attrs_json: attrs_json.to_string(),
    };
    ACTIVE.with(|active| match active.borrow().as_ref() {
        Some(harness) => harness.logs.borrow_mut().push(record),
        // Logging remains a no-op outside the harness, including in ordinary cargo tests.
        None => eprintln!("[guest-sdk log/{level:?}] {message} {attrs_json}"),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{connector, execution};

    #[test]
    fn connector_call_dispatches_to_registered_handler() {
        let host = TestHost::new().with_connector("stock-api", |req| {
            assert_eq!(req.connector, "stock-api");
            json_response(200, r#"{"approved":true}"#)
        });
        let response = host.run(|| {
            connector::call(ConnectorRequest {
                connector: "stock-api".to_string(),
                body: b"{}".to_vec(),
                additional_headers: Vec::new(),
                timeout_hint_ms: None,
            })
        });
        let response = response.expect("the mock returns a successful response");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, br#"{"approved":true}"#.to_vec());
    }

    #[test]
    #[should_panic(expected = "is not registered on the TestHost")]
    fn unregistered_connector_panics() {
        let host = TestHost::new();
        let _ = host.run(|| {
            connector::call(ConnectorRequest {
                connector: "unknown".to_string(),
                body: Vec::new(),
                additional_headers: Vec::new(),
                timeout_hint_ms: None,
            })
        });
    }

    #[test]
    fn panic_hook_routes_panic_message_to_logs() {
        crate::__internal::init_panic_hook();
        let host = TestHost::new();
        host.run(|| {
            let result = std::panic::catch_unwind(|| panic!("boom at checkout"));
            assert!(result.is_err());
        });
        let logs = host.logs();
        assert!(
            logs.iter().any(|log| log.level == LogLevel::Error
                && log.message == "guest panic"
                && log.attrs_json.contains("boom at checkout")
                && log.attrs_json.contains("testing.rs")),
            "the panic log is missing: {logs:?}"
        );
    }

    #[cfg(feature = "order-completed-v1")]
    #[test]
    fn buyer_mail_reservations_are_recorded_and_validated() {
        use crate::order_completed::mail::{self, BuyerMail, MailBodyFormat, MailErrorCode};

        let host = TestHost::new();
        host.run(|| {
            mail::send_to_buyer(&BuyerMail::text("order-thanks", "Thanks", "Thank you.\nSee you again.")).unwrap();
            let dup = mail::send_to_buyer(&BuyerMail::text("order-thanks", "Thanks", "again"));
            assert_eq!(dup.unwrap_err().code, MailErrorCode::OperationKeyInvalid);
            for key in ["", "order thanks", "order/thanks", "order-caf\u{e9}"] {
                let invalid = mail::send_to_buyer(&BuyerMail::text(key, "Thanks", "body"));
                assert_eq!(invalid.unwrap_err().code, MailErrorCode::OperationKeyInvalid, "{key:?}");
            }
            let too_long = mail::send_to_buyer(&BuyerMail::text("k".repeat(129), "Thanks", "body"));
            assert_eq!(too_long.unwrap_err().code, MailErrorCode::OperationKeyInvalid);
            let newline = mail::send_to_buyer(&BuyerMail::text("newline", "a\nb", "body"));
            assert_eq!(newline.unwrap_err().code, MailErrorCode::SubjectInvalid);
            let empty_body = mail::send_to_buyer(&BuyerMail::html("empty", "Thanks", "  \n"));
            assert_eq!(empty_body.unwrap_err().code, MailErrorCode::BodyInvalid);
            let control = mail::send_to_buyer(&BuyerMail::html("control", "Thanks", "a\u{0}b"));
            assert_eq!(control.unwrap_err().code, MailErrorCode::BodyInvalid);
            mail::send_to_buyer(&BuyerMail::html("survey", "Survey", "<p>Tell us how we did.</p>")).unwrap();
        });
        let reserved = host.buyer_mails();
        assert_eq!(reserved.len(), 2);
        assert_eq!(reserved[0].operation_key, "order-thanks");
        assert_eq!(reserved[0].format, MailBodyFormat::Text);
        assert_eq!(reserved[1].format, MailBodyFormat::Html);
    }

    #[cfg(feature = "order-completed-v1")]
    #[test]
    fn buyer_mail_subject_follows_the_runtime_rules() {
        use crate::order_completed::mail::{self, BuyerMail, MailErrorCode};

        let longest = "\u{3042}".repeat(MAX_BUYER_MAIL_SUBJECT_CHARS);
        let accepted = [
            "Thanks".to_string(),
            longest.clone(),
            // Zero width joiner keeps emoji sequences intact.
            "Thanks \u{1F468}\u{200D}\u{1F469}".to_string(),
        ];
        let rejected = [
            ("".to_string(), "subject must not be empty".to_string()),
            ("   ".to_string(), "subject must not be empty".to_string()),
            ("\u{3000}\u{A0}".to_string(), "subject must not be empty".to_string()),
            (format!("{longest}a"), "subject exceeds 200 characters (got 201)".to_string()),
        ];
        let undisplayable = [
            '\u{0}', '\n', '\u{7F}', '\u{85}', '\u{200B}', '\u{200E}', '\u{200F}', '\u{FEFF}', '\u{202A}', '\u{202E}',
            '\u{2066}', '\u{2069}',
        ];

        let host = TestHost::new();
        host.run(|| {
            for (index, subject) in accepted.iter().enumerate() {
                mail::send_to_buyer(&BuyerMail::text(format!("ok-{index}"), subject.clone(), "body"))
                    .unwrap_or_else(|err| panic!("{subject:?}: {err:?}"));
            }
        });
        for (subject, message) in rejected {
            let err = TestHost::new()
                .run(|| mail::send_to_buyer(&BuyerMail::text("subject", subject.clone(), "body")))
                .expect_err(&subject);
            assert_eq!(err.code, MailErrorCode::SubjectInvalid, "{subject:?}");
            assert_eq!(err.message, message, "{subject:?}");
        }
        for ch in undisplayable {
            let subject = format!("Thanks{ch}");
            let err = TestHost::new()
                .run(|| mail::send_to_buyer(&BuyerMail::text("subject", subject.clone(), "body")))
                .expect_err(&subject);
            assert_eq!(err.code, MailErrorCode::SubjectInvalid, "{ch:?}");
            assert_eq!(err.message, "subject must not contain control or formatting characters", "{ch:?}");
        }
    }

    #[test]
    fn execution_context_and_logs_are_harness_scoped() {
        let host = TestHost::new().with_execution_context(ExecutionContext {
            run_id: "run-42".to_string(),
            started_at_unix_ms: 1,
            deadline_unix_ms: 2,
        });
        host.run(|| {
            let context = execution::get_context();
            crate::host::info("hello", serde_json::json!({ "run_id": context.run_id }));
        });
        let logs = host.logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].level, LogLevel::Info);
        assert_eq!(logs[0].message, "hello");
        assert!(logs[0].attrs_json.contains("run-42"));
    }
}
