//! Application logging setup.

use std::io::Write;

/// Initialize `env_logger` once.
///
/// Default filter: `info` for this crate, `warn` for noisy dependencies.
/// Override with `RUST_LOG`, e.g. `RUST_LOG=ssh_pane=debug`.
pub fn init() {
    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("ssh_pane=info,warn"),
    );
    builder.format(|buf, record| {
        let ts = buf.timestamp_millis();
        writeln!(
            buf,
            "[{ts} {level:<5} {target}] {args}",
            level = record.level(),
            target = record.target(),
            args = record.args()
        )
    });
    // Ignore "already initialized" when tests call init more than once.
    let _ = builder.try_init();
}
