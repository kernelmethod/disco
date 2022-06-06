use std::time;

/// Returns the default amount of time the writer thread
/// should sleep while waiting for a new client to connect.
pub const fn default_sleep_time() -> time::Duration {
    time::Duration::from_millis(20)
}
