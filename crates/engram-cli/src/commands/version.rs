use anyhow::Result;
use engram_core::update;

pub fn run() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!("engram {current}");

    // Forced update check (synchronous, ignores cache TTL)
    if !update::is_update_check_disabled() {
        match update::check_for_update(current, true) {
            Some(info) => {
                eprintln!("{}", update::format_update_notice(&info));
            }
            None => {
                eprintln!("You are up to date.");
            }
        }
    }

    Ok(())
}
