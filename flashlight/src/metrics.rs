/// RuntimeSpec is the attributes of the runtime environment, used to
/// distinguish this metric set from others
pub(crate) struct RuntimeSpec {
    pub(crate) rpc_endpoint: String,
    pub(crate) drift_env: String,
    pub(crate) commit: String,
    pub(crate) drift_pid: String,
    pub(crate) wallet_authority: String,
}
