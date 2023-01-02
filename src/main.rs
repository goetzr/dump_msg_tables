use std::fmt;
use std::mem;
use std::ffi::c_void;

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
    Ok(())
}

#[derive(Debug)]
pub enum Error {
    GetResNames {
        mod_name: String,
        err_msg: String,
        sys_err: sys::Error,
    },
    GetResTypes {
        mod_name: String,
        err_msg: String,
        sys_err: sys::Error,
    },
    GetMsgTblEntries {
        mod_name: String,
        err_msg: String,
        sys_err: sys::Error,
    },
    ResourceMetadata,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Error::*;
        match &self {
            GetResNames {
                mod_name,
                err_msg,
                sys_err,
            } => {
                write!(
                    f,
                    "failed to get resource names for {}: {}: {}",
                    mod_name, err_msg, sys_err
                )
            }
            GetResTypes {
                mod_name,
                err_msg,
                sys_err,
            } => {
                write!(
                    f,
                    "failed to get resource types for {}: {}: {}",
                    mod_name, err_msg, sys_err
                )
            }
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
            ResourceMetadata => {
                write!(
                    f,
                    "invalid resource metadata: expected resource ID after '#'"
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

pub fn get_all_message_table_entries(mod_name: &str) -> Result<Vec<(u16, String)>> {
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

    for mt_res_name in mt_res_names {}
    unimplemented!()
}

fn get_message_table_entries(
    mod_name: &str,
    module: HINSTANCE,
    mt_name: ResourceName,
) -> Result<Vec<(u32, String)>> {
    let resource = sys::find_resource(
        module,
        mt_name,
        sys::ResourceType::from_id(sys::RT_MESSAGETABLE),
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

    let data = unsafe { mem::transmute::<&c_void, &MESSAGE_RESOURCE_DATA>(&*res_mem) };
    
    let blocks = unsafe { std::slice::from_raw_parts(
        &data.Blocks as *const MESSAGE_RESOURCE_BLOCK,
        data.NumberOfBlocks as usize
    )};
    for block in blocks {
        // NOTE: Each entry is variable length.
        let start_entries = unsafe {
            (data as *const MESSAGE_RESOURCE_DATA as *const u8).add(block.OffsetToEntries as usize)
        };
        let mut entry = unsafe {
            &*(start_entries as *const MESSAGE_RESOURCE_ENTRY)
        };
        let num_entries = block.HighId - block.LowId + 1;
        for entry_idx in 0..num_entries {
            let entry_id = block.LowId + entry_idx;

            entry = unsafe {
                &*((entry as *const u8).add(entry.Length))
        }
    }

    unimplemented!()
}
