#[cfg(target_arch = "wasm32")]
use {
    ic_certified_map::{leaf_hash, AsHashTree, Hash, RbTree},
    serde::Serialize,
    serde_cbor::Serializer,
    std::cell::RefCell,
};

#[cfg(target_arch = "wasm32")]
thread_local! {
    static TREE: RefCell<RbTree<Vec<u8>, Hash>> = RefCell::new(RbTree::new());
}

#[cfg(target_arch = "wasm32")]
pub fn update(principal: candid::Principal, holdings: &[bx_core::Holding]) {
    TREE.with(|t| {
        let mut tree = t.borrow_mut();
        let bytes = serde_json::to_vec(holdings).expect("serialize holdings");
        tree.insert(principal.to_text().into_bytes(), leaf_hash(&bytes));
        ic_cdk::api::set_certified_data(&tree.root_hash());
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn update(_principal: candid::Principal, _holdings: &[bx_core::Holding]) {}

#[cfg(target_arch = "wasm32")]
pub fn witness(principal: candid::Principal) -> Vec<u8> {
    TREE.with(|t| {
        let tree = t.borrow();
        let mut out = Vec::new();
        let mut ser = Serializer::new(&mut out);
        let _ = ser.self_describe();
        tree.witness(principal.to_text().as_bytes())
            .serialize(&mut ser)
            .expect("serialize witness");
        out
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn witness(_principal: candid::Principal) -> Vec<u8> {
    Vec::new()
}
