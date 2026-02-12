use anyhow::Result;

pub fn run() -> Result<()> {
    println!("engram {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
