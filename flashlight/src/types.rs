pub(crate) trait Bot {
    /// Initialize the bot
    async fn init();

    /// Reset the bot. This is called to reset the bot to a fresh state (pre-init).
    async fn reset();

    /// Start the bot loop. This is generally a polling loop.
    async fn start_interval_loop(interval_ms: u16);

    /// Returns true if bot is healthy, else false. Typically used for monitoring liveness.
    async fn health_check() -> bool;
}

pub(crate) enum JitoStrategy {
    JitoOnly,
    NonJitoOnly,
    Hybrid,
}
