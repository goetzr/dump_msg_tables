use std::ffi::c_void;
use std::fmt;
use std::mem;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

mod sys;

use sys::{ResourceName, ResourceType};

fn main() {
    if let Err(e) = try_main() {
        println!("ERROR: {}", e);
        std::process::exit(1);
    }
}

fn try_main() -> anyhow::Result<()> {
    let entries = get_message_table_entries("ping.exe")?;
    for entry in entries {
        println!("{:08x}: {}", entry.0, entry.1);
    }
    Ok(())
}

#[derive(Debug)]
enum Error {
    GetMsgTblEntries {
        mod_name: String,
        err_msg: String,
        sys_err: sys::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Error::*;
        match &self {
            GetMsgTblEntries {
                mod_name,
                err_msg,
                sys_err,
            } => {
                write!(
                    f,
                    "failed to get message table entries for {}: {}: {}",
                    mod_name, err_msg, sys_err
                )
            }
        }
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
        Err(_) => println!("ERROR: invalid resource name: expected ID after '#'"),
    };

    true.into()
}

fn get_message_table_entries(mod_name: &str) -> Result<Vec<(u32, String)>> {
    let module = sys::load_library(mod_name).map_err(|e| Error::GetMsgTblEntries {
        mod_name: mod_name.to_string(),
        err_msg: "failed to load the module".to_string(),
        sys_err: e,
    })?;

    let mut mt_res_names: Vec<ResourceName> = Vec::new();
    let param = unsafe { mem::transmute::<&mut Vec<ResourceName>, isize>(&mut mt_res_names) };
    sys::enum_resource_names(
        module,
        sys::ResourceType::from_id(sys::RT_MESSAGETABLE),
        Some(enum_res_names),
        param,
    )
    .map_err(|e| Error::GetMsgTblEntries {
        mod_name: mod_name.to_string(),
        err_msg: "failed to enumerate message table resource names".to_string(),
        sys_err: e,
    })?;

    let mut results = Vec::new();
    for mt_res_name in mt_res_names {
        results.extend(get_message_table_entries_inner(mod_name, module, mt_res_name)?)
    }
    Ok(results)
}

fn get_message_table_entries_inner(
    mod_name: &str,
    module: HINSTANCE,
    mt_res_name: ResourceName,
) -> Result<Vec<(u32, String)>> {
    let resource = sys::find_resource(
        module,
        mt_res_name,
        ResourceType::from_id(sys::RT_MESSAGETABLE),
    )
    .map_err(|e| Error::GetMsgTblEntries {
        mod_name: mod_name.to_string(),
        err_msg: "failed to find the resource".to_string(),
        sys_err: e,
    })?;

    let res_data = sys::load_resource(module, resource).map_err(|e| Error::GetMsgTblEntries {
        mod_name: mod_name.to_string(),
        err_msg: "failed to load the resource".to_string(),
        sys_err: e,
    })?;

    let res_mem = sys::lock_resource(res_data).map_err(|e| Error::GetMsgTblEntries {
        mod_name: mod_name.to_string(),
        err_msg: "failed to lock the resource".to_string(),
        sys_err: e,
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
                0 => sys::ansi_to_utf8(entry.Text.as_ptr()),
                // Unicode
                1 => sys::wide_to_utf8(entry.Text.as_ptr() as  *const u16),
                _ => panic!("Unexpected flags value in message table entry"),
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
