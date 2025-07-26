#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    Network(String),
    InvalidConfig(String),
    InvalidResponse,
}

impl core::fmt::Display for FetchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "network error: {e}"),
            Self::InvalidConfig(name) => write!(f, "invalid config: {name}"),
            Self::InvalidResponse => f.write_str("invalid response"),
        }
    }
}

impl std::error::Error for FetchError {}

#[cfg(not(target_arch = "wasm32"))]
impl From<ic_agent::AgentError> for FetchError {
    fn from(e: ic_agent::AgentError) -> Self {
        FetchError::Network(e.to_string())
    }
}
