use std::ffi::c_void;
use std::fmt;
use std::mem;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use window_polish::{string, error, module, resource::{self, ResourceName}};

fn main() {
    if let Err(e) = try_main() {
        println!("ERROR: {}", e);
        std::process::exit(1);
    }
}

fn try_main() -> anyhow::Result<()> {
    // TODO: Pass module path on the command line.
    let entries = get_message_table_entries("ping.exe")?;
    for entry in entries {
        println!("{:>8x}: {}", entry.0, entry.1);
    }
    Ok(())
}

#[derive(Debug)]
struct Error {
    err_msg: String,
    win_err: error::Error,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to get message table entries{}: {}",
            self.err_msg, self.win_err
        )
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

unsafe extern "system" fn enum_res_names(
    _module: HINSTANCE,
    _typ: PCWSTR,
    name: PCWSTR,
    param: isize,
) -> BOOL {
    let names = mem::transmute::<isize, &mut Vec<ResourceName>>(param);

    match ResourceName::parse(name) {
        Ok(res_name) => names.push(res_name),
        Err(_) => println!("ERROR: failed to parse resource name"),
    };

    true.into()
}

fn get_message_table_entries(mod_name: &str) -> Result<Vec<(u32, String)>> {
    let module = module::LoadLibraryW(mod_name).map_err(|e| Error {
        err_msg: "failed to load the module".to_string(),
        win_err: e,
    })?;

    let mut mt_res_names: Vec<ResourceName> = Vec::new();
    let param = unsafe { mem::transmute::<&mut Vec<ResourceName>, isize>(&mut mt_res_names) };
    resource::EnumResourceNamesW(
        module,
        RT_MESSAGETABLE,
        Some(enum_res_names),
        param,
    )
    .map_err(|e| Error {
        err_msg: "failed to enumerate message table resource names".to_string(),
        win_err: e,
    })?;

    let mut results = Vec::new();
    for mt_res_name in mt_res_names {
        results.extend(get_message_table_entries_inner(module, mt_res_name)?)
    }
    Ok(results)
}

fn get_message_table_entries_inner(
    module: HINSTANCE,
    mt_res_name: ResourceName,
) -> Result<Vec<(u32, String)>> {
    let resource = resource::FindResourceW(
        module,
        &mt_res_name,
        RT_MESSAGETABLE,
    )
    .map_err(|e| Error {
        err_msg: "failed to find the resource".to_string(),
        win_err: e,
    })?;

    let res_data = resource::LoadResource(module, resource).map_err(|e| Error {
        err_msg: "failed to load the resource".to_string(),
        win_err: e,
    })?;

    let res_mem = resource::LockResource(res_data).map_err(|e| Error {
        err_msg: "failed to lock the resource".to_string(),
        win_err: e,
    })?;

    let data = unsafe { mem::transmute::<*const c_void, &MESSAGE_RESOURCE_DATA>(res_mem) };

    let mut results = Vec::new();

    let blocks = unsafe {
        std::slice::from_raw_parts(
            &data.Blocks as *const MESSAGE_RESOURCE_BLOCK,
            data.NumberOfBlocks as usize,
        )
    };
    for block in blocks {
        // NOTE: Each entry is variable length.
        let start_entries = unsafe {
            (data as *const MESSAGE_RESOURCE_DATA as *const u8).add(block.OffsetToEntries as usize)
        };
        let mut entry = unsafe { &*(start_entries as *const MESSAGE_RESOURCE_ENTRY) };
        for entry_id in block.LowId..block.HighId + 1 {
            let entry_str = match entry.Flags {
                // Ansi
                0 => string::ansi_to_utf8(entry.Text.as_ptr()),
                // Unicode
                1 => string::utf16_to_utf8(entry.Text.as_ptr() as  *const u16),
                _ => panic!("unexpected flags value in message table entry"),
            };

            results.push((entry_id, entry_str));

            unsafe {
                let next_entry = (entry as *const MESSAGE_RESOURCE_ENTRY as *const u8)
                    .add(entry.Length as usize);
                entry = &*(next_entry as *const MESSAGE_RESOURCE_ENTRY);
            }
        }
    }

    Ok(results)
}
