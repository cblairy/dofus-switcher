use anyhow::{Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;

pub(crate) fn find_dofus_windows(conn: &impl Connection) -> Result<Vec<Window>> {
    let root = conn.setup().roots[0].root;
    let client_list_atom = conn.intern_atom(false, b"_NET_CLIENT_LIST")?.reply()?.atom;
    let client_list = conn
        .get_property(false, root, client_list_atom, AtomEnum::WINDOW, 0, u32::MAX)?
        .reply()?;
    let windows: Vec<Window> = client_list
        .value32()
        .map(|values| values.collect())
        .unwrap_or_default();

    let windows = if windows.is_empty() {
        conn.query_tree(root)?.reply()?.children
    } else {
        windows
    };

    windows
        .into_iter()
        .try_fold(Vec::new(), |mut found, window| {
            if has_dofus_identity(conn, window)? {
                found.push(window);
            }
            Ok(found)
        })
}

fn has_dofus_identity(conn: &impl Connection, window: Window) -> Result<bool> {
    if read_window_property(conn, window, b"WM_CLASS")?.is_some_and(|value| is_dofus_class(&value))
    {
        return Ok(true);
    }

    let Some(pid) = read_window_property_u32(conn, window, b"_NET_WM_PID")? else {
        return Ok(false);
    };
    Ok(process_name_for_pid(pid)?.is_some_and(|name| is_dofus_executable_name(&name)))
}

fn read_window_property(
    conn: &impl Connection,
    window: Window,
    property_name: &[u8],
) -> Result<Option<Vec<u8>>> {
    let atom = conn.intern_atom(false, property_name)?.reply()?.atom;
    let property = conn
        .get_property(false, window, atom, AtomEnum::ANY, 0, 1024)?
        .reply()?;
    Ok((!property.value.is_empty()).then_some(property.value))
}

fn read_window_property_u32(
    conn: &impl Connection,
    window: Window,
    property_name: &[u8],
) -> Result<Option<u32>> {
    let atom = conn.intern_atom(false, property_name)?.reply()?.atom;
    let property = conn
        .get_property(false, window, atom, AtomEnum::CARDINAL, 0, 1)?
        .reply()?;
    Ok(property.value32().and_then(|mut values| values.next()))
}

fn is_dofus_class(value: &[u8]) -> bool {
    value
        .split(|byte| *byte == 0)
        .filter_map(|part| std::str::from_utf8(part).ok())
        .any(is_dofus_executable_name)
}

fn process_name_for_pid(pid: u32) -> Result<Option<String>> {
    let process_dir = std::path::PathBuf::from("/proc").join(pid.to_string());
    let executable_path = process_dir.join("exe");
    match std::fs::read_link(&executable_path) {
        Ok(path) => {
            return Ok(path
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .map(str::to_owned));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {}
        Err(error) => {
            return Err(error).with_context(|| format!("Reading {}", executable_path.display()));
        }
    }

    let command_path = process_dir.join("comm");
    match std::fs::read_to_string(&command_path) {
        Ok(name) => Ok(Some(name.trim().to_owned())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("Reading {}", command_path.display())),
    }
}

fn is_dofus_executable_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "dofus" | "dofus.exe" | "dofus.x64" | "dofus.x86_64"
    )
}

#[cfg(test)]
mod tests {
    use super::{is_dofus_class, is_dofus_executable_name};

    #[test]
    fn identifies_dofus_window_class() {
        assert!(is_dofus_class(b"Dofus.x64\0Dofus.x64"));
        assert!(!is_dofus_class(b"code\0Visual Studio Code"));
        assert!(!is_dofus_class(b"workspace-dofus-switcher\0Code"));
    }

    #[test]
    fn matches_only_dofus_process_names() {
        assert!(is_dofus_executable_name("Dofus"));
        assert!(is_dofus_executable_name("dofus.exe"));
        assert!(is_dofus_executable_name("Dofus.x64"));
        assert!(!is_dofus_executable_name("code"));
        assert!(!is_dofus_executable_name("dofus-switcher"));
    }
}
