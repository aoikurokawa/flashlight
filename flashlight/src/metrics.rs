/// RuntimeSpec is the attributes of the runtime environment, used to
/// distinguish this metric set from others
pub(crate) struct RuntimeSpec {
    rpc_endpoint: String,
    drift_env: String,
    commit: String,
    drift_pid: String,
    wallet_authority: String,
}
