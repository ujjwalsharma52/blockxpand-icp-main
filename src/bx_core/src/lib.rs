#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, candid::CandidType, Eq, PartialEq)]
pub struct Holding {
    pub source: String,
    pub token: String,
    pub amount: String,
    pub status: String,
}
