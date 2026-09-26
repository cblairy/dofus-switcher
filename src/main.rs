// main.rs
mod autoswitch;
mod capture;
mod shortcut_biding;
pub mod window;

use anyhow::Result;
use std::thread;

fn main() -> Result<()> {
    let (conn, _) = x11rb::connect(None)?;

    thread::spawn(move || {
        autoswitch::auto_switch(&conn);
    })
    .join()
    .map_err(|_| anyhow::anyhow!("Autoswitch thread Panic"))?;

    Ok(())
}
