// main.rs
mod capture;
mod autoswitch;
mod shortcut_biding;

use anyhow::Result;
use std::thread;

fn main() -> Result<()> {
    let (conn, _) = x11rb::connect(None)?;
    let window: u32 = 0x4c00008; // en dur pour CE test uniquement => TODO detecter les fenetres dofus

    thread::spawn(move || {
        autoswitch::auto_switch(&conn, window);
    }).join().map_err(|_| anyhow::anyhow!("Autoswitch thread Panic"))?;

    Ok(())
}