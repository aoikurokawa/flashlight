/// RuntimeSpec is the attributes of the runtime environment, used to
/// distinguish this metric set from others
pub struct RuntimeSpec {
    pub rpc_endpoint: String,
    pub drift_env: String,
    pub commit: String,
    pub drift_pid: String,
    pub wallet_authority: String,
}

impl RuntimeSpec {
    pub fn new() -> Self {
        todo!()
    }
}
