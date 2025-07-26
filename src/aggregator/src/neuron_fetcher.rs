use bx_core::Holding;
use candid::Principal;

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(_: u64) {}

#[cfg(not(target_arch = "wasm32"))]
async fn sleep_ms(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

pub async fn fetch(_principal: Principal) -> Vec<Holding> {
    sleep_ms(7).await;
    vec![Holding {
        source: "neuron".to_string(),
        token: "ICP".to_string(),
        amount: "1200".to_string(),
        status: "locked_8y".to_string(),
    }]
}
