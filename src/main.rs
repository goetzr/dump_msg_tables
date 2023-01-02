use std::fmt;
use std::mem;

use windows::core::*;
use windows::Win32::Foundation::*;

mod sys;

fn main() {
    if let Err(e) = try_main() {
        println!("ERROR: {}", e);
        std::process::exit(1);
    }
}

fn try_main() -> anyhow::Result<()> {
    let types = mui::get_resource_types("ping.exe")?;
    for typ in types {
        println!("{}", typ);
    }
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
    println!("Callback called");
    let names = mem::transmute::<isize, &mut Vec<String>>(param);

    let name_num = mem::transmute::<PCWSTR, usize>(name);
    if name_num >> 16 == 0 {
        let id: u16 = (name_num & 0xffff) as u16;
        names.push(id.to_string());
    } else {
        names.push(sys::wide_to_utf8(name.0));
    }

    true.into()
}

pub fn get_string_resource_names(mod_name: &str) -> Result<Vec<String>> {
    let module = sys::load_library(mod_name).map_err(|e| Error::GetResNames {
        mod_name: mod_name.to_string(),
        err_msg: "failed to load the module".to_string(),
        sys_err: e,
    })?;

    let mut names: Vec<String> = Vec::new();
    let param = unsafe { mem::transmute::<&mut Vec<String>, isize>(&mut names) };
    sys::enum_resource_names(module, sys::RT_MANIFEST, Some(enum_res_names), param).map_err(
        |e| Error::GetResNames {
            mod_name: mod_name.to_string(),
            err_msg: "failed to enumerate string resource names".to_string(),
            sys_err: e,
        },
    )?;

    Ok(names)
}

unsafe extern "system" fn enum_res_types(_module: HINSTANCE, typ: PCWSTR, param: isize) -> BOOL {
    let types = mem::transmute::<isize, &mut Vec<u16>>(param);

    let type_num = mem::transmute::<PCWSTR, usize>(typ);
    let type_val: u16;
    if type_num >> 16 == 0 {
        type_val = (type_num & 0xffff) as u16;
    } else {
        let type_str = sys::wide_to_utf8(typ.0);
        if type_str.starts_with("#") {
            type_val = type_str[1..].parse().expect("number should following #");
        } else {
            println!("NOTE: resource type is a string not starting with #: {}", type_str);
            type_val = 0;
        }
    }

    types.push(type_val);
    true.into()
}

pub fn get_resource_types(mod_name: &str) -> Result<Vec<u16>> {
    let module = sys::load_library(mod_name).map_err(|e| Error::GetResTypes {
        mod_name: mod_name.to_string(),
        err_msg: "failed to load the module".to_string(),
        sys_err: e,
    })?;

    let mut types: Vec<u16> = Vec::new();
    let param = unsafe { mem::transmute::<&mut Vec<u16>, isize>(&mut types) };
    sys::enum_resource_types(module, Some(enum_res_types), param).map_err(|e| {
        Error::GetResTypes {
            mod_name: mod_name.to_string(),
            err_msg: "failed to enumerate resource types".to_string(),
            sys_err: e,
        }
    })?;

    Ok(types)
}

unsafe extern "system" fn enum_res_types2(_module: HINSTANCE, typ: PCWSTR, param: isize) -> BOOL {
    let types = mem::transmute::<isize, &mut Vec<ResourceType>>(param);

    match ResourceType::parse(typ) {
        Ok(res_type) => types.push(res_type),
        Err(_) => println!("ERROR: invalid resource type: expected resource ID after '#'"),
    };

    true.into()
}

unsafe extern "system" fn enum_res_names2(
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

pub fn get_message_table_entries(mod_name: &str) -> Result<Vec<(u16, String)>> {
    let module = sys::load_library(mod_name).map_err(|e| Error::GetMsgTblEntries {
        mod_name: mod_name.to_string(),
        err_msg: "failed to load the module".to_string(),
        sys_err: e,
    })?;

    let mut mt_res_names: Vec<ResourceName> = Vec::new();
    let param = unsafe { mem::transmute::<&mut Vec<ResourceName>, isize>(&mut mt_res_names) };
    sys::enum_resource_names(module, sys::RT_MESSAGETABLE, Some(enum_res_names2), param).map_err(
        |e| Error::GetMsgTblEntries {
            mod_name: mod_name.to_string(),
            err_msg: "failed to enumerate message table resource names".to_string(),
            sys_err: e,
        },
    )?;

    for mt_res_name in mt_res_names {

    }
}

fn get_message_table_entries(module: HINSTANCE, mt_name: ResourceName) -> Result<(u16, String)>> {
    FindResourceW(module, )
}