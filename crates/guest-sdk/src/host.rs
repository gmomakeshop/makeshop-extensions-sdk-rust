//! Host APIs provided by the runtime.
//!
//! These APIs provide connector calls, execution context, UUID generation, and runtime logging.
//! In native builds, [`crate::testing::TestHost`] supplies their behavior.

/// Format of a connector response body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadFormat {
    /// A JSON response body.
    Json,
    /// An XML response body.
    Xml,
    /// Text content.
    Text,
    /// Binary content.
    Binary,
}

/// An HTTP header in a connector request or response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// Header name.
    pub name: String,
    /// Header value.
    pub value: String,
}

/// A request passed to a connector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorRequest {
    /// Connector name defined in the deployment configuration.
    pub connector: String,
    /// Request body. Its format is specified by the `Content-Type` header.
    pub body: Vec<u8>,
    /// HTTP headers added to those in the deployment configuration.
    pub additional_headers: Vec<Header>,
    /// Requested timeout in milliseconds. `None` uses the configured default.
    pub timeout_hint_ms: Option<u32>,
}

/// Validation error for a [`ConnectorRequest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorRequestValidationError {
    message: String,
}

impl core::fmt::Display for ConnectorRequestValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.message)
    }
}

impl core::error::Error for ConnectorRequestValidationError {}

impl ConnectorRequest {
    /// Creates a [`ConnectorRequestBuilder`] for the named connector.
    pub fn builder(connector: impl Into<String>) -> ConnectorRequestBuilder {
        ConnectorRequestBuilder {
            request: Self {
                connector: connector.into(),
                body: Vec::new(),
                additional_headers: Vec::new(),
                timeout_hint_ms: None,
            },
        }
    }

    /// Validates the connector name, timeout, and HTTP headers in this request.
    pub fn validate(&self) -> Result<(), ConnectorRequestValidationError> {
        let connector = self.connector.trim();
        if connector.is_empty() || connector.len() > 64 {
            return Err(validation_error("connector must be 1..=64 bytes"));
        }
        if connector != self.connector {
            return Err(validation_error("connector must not have surrounding whitespace"));
        }
        if !connector
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(validation_error("connector must match [A-Za-z0-9_-]+"));
        }
        if self.timeout_hint_ms == Some(0) {
            return Err(validation_error("timeout_hint_ms must be > 0 when specified"));
        }
        for header in &self.additional_headers {
            if !is_http_token(&header.name) {
                return Err(validation_error("header name is not an RFC 9110 token"));
            }
            if !header
                .value
                .bytes()
                .all(|byte| byte == b'\t' || (b' '..=b'~').contains(&byte))
            {
                return Err(validation_error("header value must contain only HTTP field-value characters"));
            }
        }
        Ok(())
    }
}

fn validation_error(message: &str) -> ConnectorRequestValidationError {
    ConnectorRequestValidationError {
        message: message.to_string(),
    }
}

fn is_http_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

/// Builder that validates a [`ConnectorRequest`] as it is constructed.
pub struct ConnectorRequestBuilder {
    request: ConnectorRequest,
}

impl ConnectorRequestBuilder {
    /// Sets the request body.
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.request.body = body.into();
        self
    }

    /// Adds one HTTP header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.request.additional_headers.push(Header {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    /// Sets all additional HTTP headers.
    pub fn additional_headers(mut self, headers: Vec<Header>) -> Self {
        self.request.additional_headers = headers;
        self
    }

    /// Sets the requested timeout in milliseconds.
    pub fn timeout_hint_ms(mut self, timeout_hint_ms: u32) -> Self {
        self.request.timeout_hint_ms = Some(timeout_hint_ms);
        self
    }

    /// Validates and returns the request.
    pub fn build(self) -> Result<ConnectorRequest, ConnectorRequestValidationError> {
        self.request.validate()?;
        Ok(self.request)
    }
}

/// A response returned by a connector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response body format.
    pub format: PayloadFormat,
    /// Response body.
    pub body: Vec<u8>,
    /// Response headers.
    pub headers: Vec<Header>,
    /// Trace identifier, such as a request ID returned by the upstream service.
    pub upstream_trace_id: Option<String>,
}

/// Reason a connector call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorErrorCode {
    /// The requested connector is not configured.
    NotFound,
    /// The call did not finish within its deadline. The upstream may have processed the
    /// request already, so retrying is only safe for an idempotent operation.
    Timeout,
    /// The request never reached the upstream application: name resolution, the TCP
    /// connection or the TLS handshake failed, or a secret the connector refers to could
    /// not be resolved. Retrying is safe even when the operation is not idempotent.
    ConnectError,
    /// The connection broke after the request was sent, including a failure while reading
    /// the response. The upstream may have processed the request already, so retrying is
    /// only safe for an idempotent operation.
    NetworkError,
    /// The request payload is invalid.
    PayloadInvalid,
    /// The response exceeded the size limit.
    ResponseTooLarge,
    /// The concurrency limit was reached.
    ConcurrencyLimited,
    /// The connector call limit was exceeded.
    ConnectorLimitExceeded,
}

/// An error produced by a connector call that produced no HTTP response.
///
/// An HTTP response is never an error, whatever its status. A 4xx or 5xx arrives as
/// [`ConnectorResponse`], and its `status` field carries the code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorError {
    /// Failure category.
    pub code: ConnectorErrorCode,
    /// Failure details.
    pub message: String,
    /// The status the upstream returned, when its response headers arrived before the call
    /// failed. That happens while reading the response: it exceeded the size limit, the
    /// connection broke, or the deadline passed. `None` whenever no response reached the
    /// runtime, which is every other failure.
    ///
    /// This reports what happened; it does not say the call may be re-sent. Read
    /// [`ConnectorError::code`] for that.
    pub upstream_status: Option<u16>,
}

/// Metadata for a single guest extension run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionContext {
    /// Identifier for the run.
    pub run_id: String,
    /// Run start time as Unix time in milliseconds.
    pub started_at_unix_ms: u64,
    /// Run deadline as Unix time in milliseconds.
    pub deadline_unix_ms: u64,
}

/// Runtime log level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Debug information.
    Debug,
    /// General execution information.
    Info,
    /// A condition that requires attention.
    Warn,
    /// An error that prevents processing from continuing.
    Error,
}

/// A runtime log captured by [`crate::testing::TestHost`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    /// Log level.
    pub level: LogLevel,
    /// Log message.
    pub message: String,
    /// JSON string containing the log attributes.
    pub attrs_json: String,
}

/// APIs for calling connectors.
pub mod connector {
    use super::{ConnectorError, ConnectorRequest, ConnectorResponse, Header};

    /// Executes a single connector request.
    pub fn call(req: ConnectorRequest) -> Result<ConnectorResponse, ConnectorError> {
        req.validate().map_err(|error| ConnectorError {
            code: super::ConnectorErrorCode::PayloadInvalid,
            message: error.to_string(),
            upstream_status: None,
        })?;
        super::backend::connector_call(req)
    }

    /// Executes connector requests concurrently.
    ///
    /// Results remain in request order and contain either a response or an error for each request.
    /// Use sequential [`call`] invocations when a request depends on an earlier result.
    pub fn call_batch(reqs: Vec<ConnectorRequest>) -> Vec<Result<ConnectorResponse, ConnectorError>> {
        let mut valid = Vec::new();
        let mut slots: Vec<Option<Result<ConnectorResponse, ConnectorError>>> = (0..reqs.len()).map(|_| None).collect();
        for (index, request) in reqs.into_iter().enumerate() {
            match request.validate() {
                Ok(()) => valid.push((index, request)),
                Err(error) => {
                    slots[index] = Some(Err(ConnectorError {
                        code: super::ConnectorErrorCode::PayloadInvalid,
                        message: error.to_string(),
                        upstream_status: None,
                    }));
                }
            }
        }
        let (indices, valid_requests): (Vec<_>, Vec<_>) = valid.into_iter().unzip();
        for (index, result) in indices
            .into_iter()
            .zip(super::backend::connector_call_batch(valid_requests))
        {
            slots[index] = Some(result);
        }
        slots
            .into_iter()
            .map(|slot| slot.expect("every connector batch slot is filled"))
            .collect()
    }

    /// Failures a [`RetryPolicy`] is allowed to re-send.
    ///
    /// Only `connect_error` is on by default. Every other failure may have been processed by
    /// the upstream already, so re-sending it is safe only when the operation is idempotent —
    /// something only the caller knows.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RetryOn {
        /// The request never reached the upstream application.
        /// Safe to re-send even when the operation is not idempotent.
        pub connect_error: bool,
        /// The call did not finish within its deadline.
        pub timeout: bool,
        /// The connection broke after the request was sent.
        pub network_error: bool,
        /// The upstream answered with a 5xx status.
        pub server_error: bool,
        /// The upstream answered with 429.
        pub too_many_requests: bool,
    }

    impl Default for RetryOn {
        fn default() -> Self {
            Self {
                connect_error: true,
                timeout: false,
                network_error: false,
                server_error: false,
                too_many_requests: false,
            }
        }
    }

    /// How [`call_with_retry`] re-sends a request.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RetryPolicy {
        /// Total attempts, including the first one. `1` disables retrying.
        pub max_attempts: u32,
        /// Backoff before the second attempt, in milliseconds. It doubles per attempt.
        pub initial_backoff_ms: u32,
        /// Upper bound for a single backoff, in milliseconds.
        pub max_backoff_ms: u32,
        /// Failures to re-send.
        pub retry_on: RetryOn,
    }

    impl Default for RetryPolicy {
        fn default() -> Self {
            Self {
                max_attempts: 3,
                initial_backoff_ms: 100,
                max_backoff_ms: 1_000,
                retry_on: RetryOn::default(),
            }
        }
    }

    /// Calls a connector, re-sending the request while the policy allows it.
    ///
    /// Attempts stop as soon as one of these holds:
    ///
    /// - the outcome is not one the policy re-sends,
    /// - `max_attempts` is reached,
    /// - the run has no time left for the backoff plus another attempt.
    ///
    /// The last outcome is returned as-is, so a 5xx that ran out of attempts still arrives
    /// as `Ok` carrying its status.
    ///
    /// # Choosing what to re-send
    ///
    /// Only [`RetryOn::connect_error`] is safe without further thought: the request never
    /// reached the upstream application. A timeout, a broken connection or a 5xx may have
    /// been processed already, and re-sending a non-idempotent operation then duplicates it.
    /// Enable those only for an endpoint you know is idempotent.
    ///
    /// # Time budget
    ///
    /// Backoff spends the run deadline. Before waiting, the remaining time is compared
    /// against the backoff plus how long the previous attempt took; when it does not fit,
    /// the last outcome is returned instead of starting an attempt that cannot finish.
    ///
    /// ```no_run
    /// use makeshop_extensions::host::ConnectorRequest;
    /// use makeshop_extensions::host::connector::{RetryOn, RetryPolicy, call_with_retry};
    ///
    /// let req = ConnectorRequest::builder("erp")
    ///     .body(br#"{"member_id":"m-1"}"#.to_vec())
    ///     .build()
    ///     .expect("valid request");
    ///
    /// // The endpoint is a read, so re-sending it is safe.
    /// let policy = RetryPolicy {
    ///     retry_on: RetryOn {
    ///         timeout: true,
    ///         server_error: true,
    ///         ..RetryOn::default()
    ///     },
    ///     ..RetryPolicy::default()
    /// };
    /// let res = call_with_retry(req, policy)?;
    /// # Ok::<(), makeshop_extensions::host::ConnectorError>(())
    /// ```
    pub fn call_with_retry(req: ConnectorRequest, policy: RetryPolicy) -> Result<ConnectorResponse, ConnectorError> {
        let attempts = policy.max_attempts.max(1);
        let deadline_unix_ms = super::execution::get_context().deadline_unix_ms;

        for attempt in 1..=attempts {
            let started_unix_ms = now_unix_ms();
            let outcome = call(req.clone());
            let last_attempt_ms = now_unix_ms().saturating_sub(started_unix_ms);

            let Some(reason) = retry_reason(&outcome, policy.retry_on) else {
                return outcome;
            };
            if attempt == attempts {
                return outcome;
            }

            let backoff_ms = backoff_ms(attempt, policy);
            // Do not wait when the next attempt would not fit inside the deadline. Returning the
            // last outcome now leaves the caller more options than waiting only to be cut off by
            // the host timeout.
            let remaining_ms = deadline_unix_ms.saturating_sub(now_unix_ms());
            if remaining_ms <= u64::from(backoff_ms).saturating_add(last_attempt_ms) {
                return outcome;
            }

            super::warn(
                "connector call retried",
                serde_json::json!({
                    "connector": req.connector,
                    "attempt": attempt,
                    "reason": reason,
                    "backoff_ms": backoff_ms,
                }),
            );
            sleep_ms(backoff_ms);
        }

        unreachable!("the loop returns on the last attempt")
    }

    /// Returns why the call is re-sent, or `None` when it is not.
    fn retry_reason(outcome: &Result<ConnectorResponse, ConnectorError>, retry_on: RetryOn) -> Option<&'static str> {
        use super::ConnectorErrorCode as Code;

        match outcome {
            Ok(response) => match response.status {
                429 if retry_on.too_many_requests => Some("too-many-requests"),
                500..=599 if retry_on.server_error => Some("server-error"),
                _ => None,
            },
            Err(error) => match error.code {
                Code::ConnectError if retry_on.connect_error => Some("connect-error"),
                Code::Timeout if retry_on.timeout => Some("timeout"),
                Code::NetworkError if retry_on.network_error => Some("network-error"),
                // The rest are not resolved by re-sending: a misconfiguration (NotFound), a request
                // the guest built wrong (PayloadInvalid), an exceeded limit (ResponseTooLarge /
                // ConnectorLimitExceeded), and ConcurrencyLimited, which already waited out the deadline.
                _ => None,
            },
        }
    }

    /// Returns how long to wait after attempt `attempt` (1-based), grown exponentially and spread
    /// with full jitter.
    ///
    /// The jitter keeps runs that failed at the same moment from re-sending in step and hitting the
    /// upstream together. The randomness comes from the host UUID v4, so no extra capability is needed.
    fn backoff_ms(attempt: u32, policy: RetryPolicy) -> u32 {
        let exponential = policy
            .initial_backoff_ms
            .saturating_mul(1u32.checked_shl(attempt - 1).unwrap_or(u32::MAX))
            .min(policy.max_backoff_ms);
        if exponential == 0 {
            return 0;
        }
        // Take 32 bits out of the hex digits of a UUID v4. How the digits are laid out is up to the
        // host; all this needs is a value that differs from run to run.
        let seed = super::uuid::generate_uuid_v4()
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .take(8)
            .fold(0u32, |acc, c| {
                acc.wrapping_mul(16)
                    .wrapping_add(c.to_digit(16).unwrap_or(0))
            });
        seed % (exponential + 1)
    }

    fn now_unix_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(0)
    }

    fn sleep_ms(ms: u32) {
        if ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(u64::from(ms)));
        }
    }

    /// Options for [`call_json_with`].
    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    pub struct CallJsonOptions {
        /// HTTP headers to add to the request.
        pub additional_headers: Vec<Header>,
        /// Requested timeout in milliseconds.
        pub timeout_hint_ms: Option<u32>,
    }

    /// An error produced by a JSON connector call.
    #[derive(Debug)]
    pub enum CallJsonError {
        /// The request body could not be encoded as JSON.
        Encode(String),
        /// The connector call failed.
        Connector(ConnectorError),
        /// The connector returned a non-2xx HTTP status.
        UnexpectedStatus {
            /// HTTP status code.
            status: u16,
            /// Response body before decoding.
            body: Vec<u8>,
        },
        /// The body of a 2xx response could not be decoded from JSON.
        Decode {
            /// HTTP status code.
            status: u16,
            /// Details of the decoding error.
            message: String,
        },
    }

    impl core::fmt::Display for CallJsonError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::Encode(message) => write!(f, "failed to encode the request body as JSON: {message}"),
                Self::Connector(err) => {
                    write!(f, "connector call failed ({:?}): {}", err.code, err.message)
                }
                Self::UnexpectedStatus {
                    status,
                    ..
                } => {
                    write!(f, "connector returned an unexpected status: status={status}")
                }
                Self::Decode {
                    status,
                    message,
                } => {
                    write!(f, "failed to decode the connector response (status={status}): {message}")
                }
            }
        }
    }

    impl core::error::Error for CallJsonError {}

    /// Calls a connector with a JSON request and response.
    ///
    /// This serializes the request, checks for a 2xx status, and deserializes the response.
    pub fn call_json<Req, Resp>(connector: &str, req: &Req) -> Result<Resp, CallJsonError>
    where
        Req: serde::Serialize + ?Sized,
        Resp: serde::de::DeserializeOwned,
    {
        call_json_with(connector, req, CallJsonOptions::default())
    }

    /// Calls a JSON connector with additional options.
    pub fn call_json_with<Req, Resp>(
        connector: &str,
        req: &Req,
        options: CallJsonOptions,
    ) -> Result<Resp, CallJsonError>
    where
        Req: serde::Serialize + ?Sized,
        Resp: serde::de::DeserializeOwned,
    {
        let body = serde_json::to_vec(req).map_err(|err| CallJsonError::Encode(err.to_string()))?;
        let mut builder = ConnectorRequest::builder(connector)
            .body(body)
            .additional_headers(options.additional_headers);
        if let Some(timeout_hint_ms) = options.timeout_hint_ms {
            builder = builder.timeout_hint_ms(timeout_hint_ms);
        }
        let request = builder.build().map_err(|error| {
            CallJsonError::Connector(ConnectorError {
                code: super::ConnectorErrorCode::PayloadInvalid,
                message: error.to_string(),
                upstream_status: None,
            })
        })?;
        let response = call(request).map_err(CallJsonError::Connector)?;
        if !(200..300).contains(&response.status) {
            return Err(CallJsonError::UnexpectedStatus {
                status: response.status,
                body: response.body,
            });
        }
        serde_json::from_slice(&response.body).map_err(|err| CallJsonError::Decode {
            status: response.status,
            message: err.to_string(),
        })
    }
}

/// APIs for retrieving the execution context.
pub mod execution {
    use super::ExecutionContext;

    /// Returns metadata for the current guest extension run.
    pub fn get_context() -> ExecutionContext {
        super::backend::get_context()
    }
}

/// APIs for generating UUIDs.
pub mod uuid {
    /// Returns the string representation of a version 7 UUID.
    pub fn generate_uuid_v7() -> String {
        super::backend::generate_uuid(7)
    }

    /// Returns the string representation of a version 4 UUID.
    pub fn generate_uuid_v4() -> String {
        super::backend::generate_uuid(4)
    }
}

/// Records a debug-level runtime log.
pub fn debug(message: &str, attrs: impl serde::Serialize) {
    backend::log(LogLevel::Debug, message, &to_attrs_json(&attrs));
}

/// Records an info-level runtime log.
pub fn info(message: &str, attrs: impl serde::Serialize) {
    backend::log(LogLevel::Info, message, &to_attrs_json(&attrs));
}

/// Records a warning-level runtime log.
pub fn warn(message: &str, attrs: impl serde::Serialize) {
    backend::log(LogLevel::Warn, message, &to_attrs_json(&attrs));
}

/// Records an error-level runtime log.
pub fn error(message: &str, attrs: impl serde::Serialize) {
    backend::log(LogLevel::Error, message, &to_attrs_json(&attrs));
}

// If attributes cannot be encoded as JSON, record the encoding error as an attribute instead.
fn to_attrs_json(attrs: &impl serde::Serialize) -> String {
    serde_json::to_string(attrs)
        .unwrap_or_else(|err| serde_json::json!({ "attrs_serialize_error": err.to_string() }).to_string())
}

// On wasm32, call host APIs through the bindings for the enabled entrypoint.
#[cfg(target_arch = "wasm32")]
mod backend {
    // The host capability WIT (`makeshop:runtime-host`) is shared by every contract, but bindings
    // generated from different contract worlds are nominally distinct types. Every binding reaches
    // the same host functions, so selecting one enabled entrypoint feature is sufficient.
    //
    // Generate feature selection and count validation from the same list.
    macro_rules! select_host_backend {
        ($(($feature:literal, $module:ident)),+ $(,)?) => {
            select_host_backend!(@pick [] $(($feature, $module))+);
            select_host_backend!(@require_one [] $(($feature, $module))+);
            select_host_backend!(@reject_multi $(($feature, $module))+);
        };

        // Select the first enabled feature in the list.
        (@pick [$($prev:literal)*] ($feature:literal, $module:ident) $($rest:tt)*) => {
            #[cfg(all(feature = $feature, not(any($(feature = $prev),*))))]
            use crate::$module::bindings::makeshop::runtime_host as rh;
            select_host_backend!(@pick [$($prev)* $feature] $($rest)*);
        };
        (@pick [$($prev:literal)*]) => {};

        (@require_one [$($seen:literal)*] ($feature:literal, $module:ident) $($rest:tt)*) => {
            select_host_backend!(@require_one [$($seen)* $feature] $($rest)*);
        };
        (@require_one [$($seen:literal)*]) => {
            #[cfg(not(any($(feature = $seen),*)))]
            compile_error!(
                "a wasm32 build must enable exactly one entrypoint contract feature (for example features = [\"finalize-checkout-validation-v1\"])"
            );
        };

        // Enumerate enabled feature pairs to detect duplicates.
        (@reject_multi ($feature:literal, $module:ident) $($rest:tt)*) => {
            select_host_backend!(@reject_pairs $feature, $($rest)*);
            select_host_backend!(@reject_multi $($rest)*);
        };
        (@reject_multi) => {};
        (@reject_pairs $head:literal, $(($feature:literal, $module:ident))*) => {
            #[cfg(all(feature = $head, any($(feature = $feature),*)))]
            compile_error!(
                "a wasm32 build must not enable more than one entrypoint contract feature (one guest exports one entrypoint under one contract)"
            );
        };
    }

    select_host_backend!(
        ("finalize-checkout-validation-v1", finalize_checkout_validation),
        ("order-completed-v1", order_completed),
        ("order-completed-content-v1", order_completed_content),
    );

    use super::{
        ConnectorError,
        ConnectorErrorCode,
        ConnectorRequest,
        ConnectorResponse,
        ExecutionContext,
        Header,
        LogLevel,
        PayloadFormat,
    };

    pub(super) fn connector_call(req: ConnectorRequest) -> Result<ConnectorResponse, ConnectorError> {
        rh::connector::call(&to_wit_request(req))
            .map(from_wit_response)
            .map_err(from_wit_error)
    }

    pub(super) fn connector_call_batch(reqs: Vec<ConnectorRequest>) -> Vec<Result<ConnectorResponse, ConnectorError>> {
        let wit_reqs: Vec<_> = reqs.into_iter().map(to_wit_request).collect();
        rh::connector::call_batch(&wit_reqs)
            .into_iter()
            .map(|result| result.map(from_wit_response).map_err(from_wit_error))
            .collect()
    }

    pub(super) fn get_context() -> ExecutionContext {
        let context = rh::execution::get_context();
        ExecutionContext {
            run_id: context.run_id,
            started_at_unix_ms: context.started_at_unix_ms,
            deadline_unix_ms: context.deadline_unix_ms,
        }
    }

    pub(super) fn generate_uuid(version: u8) -> String {
        match version {
            7 => rh::uuid::generate_uuid_v7(),
            _ => rh::uuid::generate_uuid_v4(),
        }
    }

    pub(super) fn log(level: LogLevel, message: &str, attrs_json: &str) {
        match level {
            LogLevel::Debug => rh::logging::debug(message, attrs_json),
            LogLevel::Info => rh::logging::info(message, attrs_json),
            LogLevel::Warn => rh::logging::warn(message, attrs_json),
            LogLevel::Error => rh::logging::error(message, attrs_json),
        }
    }

    fn from_wit_format(format: rh::connector::PayloadFormat) -> PayloadFormat {
        match format {
            rh::connector::PayloadFormat::Json => PayloadFormat::Json,
            rh::connector::PayloadFormat::Xml => PayloadFormat::Xml,
            rh::connector::PayloadFormat::Text => PayloadFormat::Text,
            rh::connector::PayloadFormat::Binary => PayloadFormat::Binary,
        }
    }

    fn to_wit_request(req: ConnectorRequest) -> rh::connector::ConnectorRequest {
        rh::connector::ConnectorRequest {
            connector: req.connector,
            body: req.body,
            additional_headers: req
                .additional_headers
                .into_iter()
                .map(|header| rh::connector::Header {
                    name: header.name,
                    value: header.value,
                })
                .collect(),
            timeout_hint_ms: req.timeout_hint_ms,
        }
    }

    fn from_wit_response(res: rh::connector::ConnectorResponse) -> ConnectorResponse {
        ConnectorResponse {
            status: res.status,
            format: from_wit_format(res.format),
            body: res.body,
            headers: res
                .headers
                .into_iter()
                .map(|header| Header {
                    name: header.name,
                    value: header.value,
                })
                .collect(),
            upstream_trace_id: res.upstream_trace_id,
        }
    }

    fn from_wit_error(err: rh::connector::ConnectorError) -> ConnectorError {
        use rh::connector::ConnectorErrorCode as Wit;
        ConnectorError {
            code: match err.code {
                Wit::NotFound => ConnectorErrorCode::NotFound,
                Wit::Timeout => ConnectorErrorCode::Timeout,
                Wit::ConnectError => ConnectorErrorCode::ConnectError,
                Wit::NetworkError => ConnectorErrorCode::NetworkError,
                Wit::PayloadInvalid => ConnectorErrorCode::PayloadInvalid,
                Wit::ResponseTooLarge => ConnectorErrorCode::ResponseTooLarge,
                Wit::ConcurrencyLimited => ConnectorErrorCode::ConcurrencyLimited,
                Wit::ConnectorLimitExceeded => ConnectorErrorCode::ConnectorLimitExceeded,
            },
            message: err.message,
            upstream_status: err.upstream_status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::connector::{CallJsonError, RetryOn, RetryPolicy, call_json, call_with_retry};
    use super::{ConnectorError, ConnectorErrorCode, ConnectorRequest};
    use crate::testing::{TestHost, json_response};

    #[derive(serde::Serialize)]
    struct Req {
        member_id: &'static str,
    }

    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct Resp {
        approved: bool,
    }

    #[test]
    fn connector_request_builder_rejects_invalid_state_before_host_call() {
        assert!(ConnectorRequest::builder("").build().is_err());
        assert!(ConnectorRequest::builder(" erp ").build().is_err());
        assert!(
            ConnectorRequest::builder("erp")
                .timeout_hint_ms(0)
                .build()
                .is_err()
        );
        assert!(
            ConnectorRequest::builder("erp")
                .header("bad header", "value")
                .build()
                .is_err()
        );
        assert!(
            ConnectorRequest::builder("erp")
                .header("x-note", "ok\r\ninjected: yes")
                .build()
                .is_err()
        );
    }

    #[test]
    fn connector_request_builder_constructs_valid_request() {
        let request = ConnectorRequest::builder("erp-v2")
            .body(br#"{"member_id":"M000001"}"#.to_vec())
            .header("content-type", "application/json")
            .timeout_hint_ms(500)
            .build()
            .expect("valid connector request");

        assert_eq!(request.connector, "erp-v2");
        assert_eq!(request.additional_headers.len(), 1);
        assert_eq!(request.timeout_hint_ms, Some(500));
    }

    #[test]
    fn call_json_decodes_2xx_response() {
        let host = TestHost::new().with_connector("erp", |req| {
            assert_eq!(req.body, br#"{"member_id":"m-1"}"#.to_vec());
            json_response(200, r#"{"approved":true}"#)
        });
        let result: Result<Resp, _> = host.run(|| {
            call_json(
                "erp",
                &Req {
                    member_id: "m-1",
                },
            )
        });
        assert_eq!(
            result.unwrap(),
            Resp {
                approved: true
            }
        );
    }

    #[test]
    fn call_json_classifies_non_2xx_as_unexpected_status() {
        let host = TestHost::new().with_connector("erp", |_req| json_response(503, "oops"));
        let result: Result<Resp, _> = host.run(|| {
            call_json(
                "erp",
                &Req {
                    member_id: "m-1",
                },
            )
        });
        match result.unwrap_err() {
            CallJsonError::UnexpectedStatus {
                status,
                body,
            } => {
                assert_eq!(status, 503);
                assert_eq!(body, b"oops".to_vec());
            }
            other => panic!("expected UnexpectedStatus: {other:?}"),
        }
    }

    #[test]
    fn call_json_classifies_broken_body_as_decode() {
        let host = TestHost::new().with_connector("erp", |_req| json_response(200, "not-json"));
        let result: Result<Resp, _> = host.run(|| {
            call_json(
                "erp",
                &Req {
                    member_id: "m-1",
                },
            )
        });
        assert!(matches!(
            result.unwrap_err(),
            CallJsonError::Decode {
                status: 200,
                ..
            }
        ));
    }

    #[test]
    fn log_attrs_serialize_failure_keeps_the_log_with_the_reason() {
        struct Broken;
        impl serde::Serialize for Broken {
            fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("attrs are broken"))
            }
        }

        let host = TestHost::new();
        host.run(|| {
            crate::host::info("still logged", Broken);
        });
        let logs = host.logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message, "still logged");
        assert!(
            logs[0].attrs_json.contains("attrs_serialize_error") && logs[0].attrs_json.contains("attrs are broken"),
            "attrs must record why serialization failed: {}",
            logs[0].attrs_json
        );
    }

    /// By default only connect-error is re-sent: the one failure that is certainly not delivered to
    /// the upstream application.
    #[test]
    fn call_with_retry_resends_connect_error_by_default() {
        let attempts = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let seen = std::rc::Rc::clone(&attempts);
        let host = TestHost::new().with_connector("erp", move |_req| {
            seen.set(seen.get() + 1);
            if seen.get() < 3 {
                return Err(ConnectorError {
                    code: ConnectorErrorCode::ConnectError,
                    message: "connection refused".to_string(),
                    upstream_status: None,
                });
            }
            json_response(200, r#"{"member_id":"m-1"}"#)
        });

        let response = host
            .run(|| call_with_retry(request(), fast_policy(RetryOn::default())))
            .expect("the third attempt succeeds");

        assert_eq!(response.status, 200);
        assert_eq!(attempts.get(), 3);
    }

    /// timeout and network-error may already have been processed upstream, so they are not re-sent by
    /// default. Only a caller that knows the call is idempotent opts in.
    #[test]
    fn call_with_retry_leaves_ambiguous_failures_alone_by_default() {
        for code in [ConnectorErrorCode::Timeout, ConnectorErrorCode::NetworkError] {
            let attempts = std::rc::Rc::new(std::cell::Cell::new(0u32));
            let seen = std::rc::Rc::clone(&attempts);
            let host = TestHost::new().with_connector("erp", move |_req| {
                seen.set(seen.get() + 1);
                Err(ConnectorError {
                    code,
                    message: "ambiguous".to_string(),
                    upstream_status: None,
                })
            });

            let error = host
                .run(|| call_with_retry(request(), fast_policy(RetryOn::default())))
                .expect_err("must not be re-sent by default");

            assert_eq!(error.code, code);
            assert_eq!(attempts.get(), 1, "{code:?} must be attempted once");
        }
    }

    /// An error that a retry cannot resolve is returned after one attempt even when opted in.
    #[test]
    fn call_with_retry_never_resends_failures_a_retry_cannot_fix() {
        let retry_everything = RetryOn {
            connect_error: true,
            timeout: true,
            network_error: true,
            server_error: true,
            too_many_requests: true,
        };
        for code in [
            ConnectorErrorCode::NotFound,
            ConnectorErrorCode::PayloadInvalid,
            ConnectorErrorCode::ResponseTooLarge,
            ConnectorErrorCode::ConcurrencyLimited,
            ConnectorErrorCode::ConnectorLimitExceeded,
        ] {
            let attempts = std::rc::Rc::new(std::cell::Cell::new(0u32));
            let seen = std::rc::Rc::clone(&attempts);
            let host = TestHost::new().with_connector("erp", move |_req| {
                seen.set(seen.get() + 1);
                Err(ConnectorError {
                    code,
                    message: "terminal".to_string(),
                    upstream_status: None,
                })
            });

            let error = host
                .run(|| call_with_retry(request(), fast_policy(retry_everything)))
                .expect_err("must surface the terminal failure");

            assert_eq!(error.code, code);
            assert_eq!(attempts.get(), 1, "{code:?} must be attempted once");
        }
    }

    /// Retrying on status still returns the last outcome as `Ok` once the attempts run out, so the
    /// caller can read the status and decide for itself.
    #[test]
    fn call_with_retry_returns_the_last_status_when_attempts_run_out() {
        let attempts = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let seen = std::rc::Rc::clone(&attempts);
        let host = TestHost::new().with_connector("erp", move |_req| {
            seen.set(seen.get() + 1);
            json_response(503, "unavailable")
        });
        let retry_on = RetryOn {
            server_error: true,
            ..RetryOn::default()
        };

        let response = host
            .run(|| call_with_retry(request(), fast_policy(retry_on)))
            .expect("a 5xx is a response, not an error");

        assert_eq!(response.status, 503);
        assert_eq!(attempts.get(), 3);
    }

    /// `max_attempts` of 1 turns re-sending off.
    #[test]
    fn call_with_retry_honours_a_single_attempt() {
        let attempts = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let seen = std::rc::Rc::clone(&attempts);
        let host = TestHost::new().with_connector("erp", move |_req| {
            seen.set(seen.get() + 1);
            Err(ConnectorError {
                code: ConnectorErrorCode::ConnectError,
                message: "connection refused".to_string(),
                upstream_status: None,
            })
        });
        let policy = RetryPolicy {
            max_attempts: 1,
            ..fast_policy(RetryOn::default())
        };

        let _ = host.run(|| call_with_retry(request(), policy));

        assert_eq!(attempts.get(), 1);
    }

    /// When the remaining time cannot cover the backoff and another attempt, the last outcome is
    /// returned without waiting.
    #[test]
    fn call_with_retry_stops_when_the_deadline_leaves_no_room() {
        let attempts = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let seen = std::rc::Rc::clone(&attempts);
        let host = TestHost::new()
            .with_execution_context(super::ExecutionContext {
                run_id: "run-out-of-time".to_string(),
                started_at_unix_ms: 0,
                deadline_unix_ms: 0,
            })
            .with_connector("erp", move |_req| {
                seen.set(seen.get() + 1);
                Err(ConnectorError {
                    code: ConnectorErrorCode::ConnectError,
                    message: "connection refused".to_string(),
                    upstream_status: None,
                })
            });

        let error = host
            .run(|| call_with_retry(request(), fast_policy(RetryOn::default())))
            .expect_err("no time is left for another attempt");

        assert_eq!(error.code, ConnectorErrorCode::ConnectError);
        assert_eq!(attempts.get(), 1, "the backoff must not be spent past the deadline");
    }

    fn request() -> ConnectorRequest {
        ConnectorRequest::builder("erp")
            .body(b"{}".to_vec())
            .build()
            .expect("valid request")
    }

    /// Collapses the backoff so the tests never actually sleep. The retry decisions do not depend on
    /// how long the backoff is, so the wait itself is not covered here.
    fn fast_policy(retry_on: RetryOn) -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            initial_backoff_ms: 0,
            max_backoff_ms: 0,
            retry_on,
        }
    }

    #[test]
    fn call_json_passes_through_connector_error() {
        let host = TestHost::new().with_connector("erp", |_req| {
            Err(ConnectorError {
                code: ConnectorErrorCode::Timeout,
                message: "upstream timeout".to_string(),
                upstream_status: None,
            })
        });
        let result: Result<Resp, _> = host.run(|| {
            call_json(
                "erp",
                &Req {
                    member_id: "m-1",
                },
            )
        });
        match result.unwrap_err() {
            CallJsonError::Connector(err) => assert_eq!(err.code, ConnectorErrorCode::Timeout),
            other => panic!("expected Connector: {other:?}"),
        }
    }
}

// In native builds, delegate behavior to TestHost.
#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::{ConnectorError, ConnectorRequest, ConnectorResponse, ExecutionContext, LogLevel};

    pub(super) fn connector_call(req: ConnectorRequest) -> Result<ConnectorResponse, ConnectorError> {
        crate::testing::dispatch_connector(req)
    }

    // TestHost processes each request in input order.
    pub(super) fn connector_call_batch(reqs: Vec<ConnectorRequest>) -> Vec<Result<ConnectorResponse, ConnectorError>> {
        reqs.into_iter()
            .map(crate::testing::dispatch_connector)
            .collect()
    }

    pub(super) fn get_context() -> ExecutionContext {
        crate::testing::dispatch_context()
    }

    pub(super) fn generate_uuid(version: u8) -> String {
        crate::testing::dispatch_uuid(version)
    }

    pub(super) fn log(level: LogLevel, message: &str, attrs_json: &str) {
        crate::testing::dispatch_log(level, message, attrs_json);
    }
}
