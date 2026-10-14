//! Provider-owned presentation policies. They never change economic identities.
pub struct Group {
    pub key: String,
    pub label: String,
}
pub trait ProviderGrouping {
    fn partition_group(&self, _kind: &str) -> Option<Group> {
        None
    }
    fn chain_group(&self, _reference: &str) -> Option<Group> {
        None
    }
}
pub fn policy(provider: &str) -> Option<&'static dyn ProviderGrouping> {
    match provider {
        "nexo" => Some(&crate::adapters::nexo::NexoGrouping),
        "evm" => Some(&crate::adapters::evm::WalletGrouping),
        _ => None,
    }
}
