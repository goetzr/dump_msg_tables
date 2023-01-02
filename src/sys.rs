use std::mem::{self, MaybeUninit};
use std::fmt;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Diagnostics::Debug::*;
use windows::Win32::System::Memory::*;
use windows::Win32::System::LibraryLoader::*;

#[derive(Debug)]
pub struct Error {
    code: u32,
    msg: String,
}

impl Error {
    fn new<S: Into<String>>(code: u32, msg: S) -> Self {
        Error { code, msg: msg.into() }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}) {}", self.code, self.msg)
    }
}

impl std::error::Error for Error {}

type Result<T> =  std::result::Result<T, Error>;

pub fn wide_to_utf8(mut data: *const u16) -> String {
    let mut out = String::new();
    unsafe {
        while *data != 0 {
            out.push(char::from_u32_unchecked(*data as u32));
            data = data.add(1);
        }
    }
    out
}

pub fn utf8_to_wide(data: &str) -> Vec<u16> {
    let mut out: Vec<u16> = data.encode_utf16().collect();
    // Add NULL terminator.
    out.push(0);
    out
}

pub enum ResourceMetadata {
    Id(u16),
    String(String),
}

impl ResourceMetadata {
    pub fn parse(data: PCWSTR) -> std::result::Result<Self, ()> {
        let data_num = unsafe { mem::transmute::<PCWSTR, usize>(data) };
        if data_num >> 16 == 0 {
            let id = (data_num & 0xffff) as u16;
            Ok(ResourceMetadata::Id(id))
        } else {
            let data_str = wide_to_utf8(data.0);
            if data_str.starts_with("#") {
                let id = data_str[1..].parse::<u16>();
                match id {
                    Ok(id) => Ok(ResourceMetadata::Id(id)),
                    Err(_) => Err(()),
                } 
            } else {
                Ok(ResourceMetadata::String(data_str))
            }
        }
    }
}

impl ToString for ResourceMetadata {
    fn to_string(&self) -> String {
        match &self {
            ResourceMetadata::Id(id) => format!("ID: {}", id),
            ResourceMetadata::String(id_str) => format!("String: {}", id_str),
        }
    }
}

pub type ResourceName = ResourceMetadata;
pub type ResourceType = ResourceMetadata;

pub fn build_error_message(error_code: u32) -> String {
    unsafe {
        let mut buf = MaybeUninit::<PWSTR>::uninit();
        let ret = FormatMessageW(
            FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM,
            None,
            error_code,
            0,
            mem::transmute::<*mut PWSTR, PWSTR>(buf.as_mut_ptr()),
            0,
            None,
        );
        match ret {
            0 => panic!("ERROR: FormatMessageW failed. Error code = {}.", GetLastError().0),
            _ => {
                let buf = buf.assume_init();
                let mut msg = wide_to_utf8(buf.0);
                // Remove any trailing whitespace.
                for _ in 0..msg.chars().rev().take_while(|&c| char::is_whitespace(c)).count() {
                    msg.pop();
                }
                if LocalFree(mem::transmute::<PWSTR, isize>(buf)) != 0 {
                    panic!("ERROR: LocalFree failed. Error code = {}.", GetLastError().0)
                } else {
                    msg
                }
            }
        }
    }
}

pub fn load_library(mod_name: &str) -> Result<HINSTANCE> {
    let mod_name = utf8_to_wide(mod_name);
    let mod_name = PCWSTR(mod_name.as_ptr());
    unsafe {
        LoadLibraryW(mod_name).map_err(|e| Error::new(e.code().0 as u32, e.message().to_string()))
    }
}

pub const _RT_STRING: u16 = 6;
pub const _RT_VERSION: u16 = 16;
pub const _RT_RCDATA: u16 = 10;
pub const RT_MANIFEST: u16 = 24;
pub const RT_MESSAGETABLE: u16 = 11;

pub fn enum_resource_names(module: HINSTANCE, typ: u16, enum_func: ENUMRESNAMEPROCW, param: isize) -> Result<()> {
    unsafe {
        let typ = mem::transmute::<u64, PCWSTR>(typ as u64);
        if EnumResourceNamesW(module, typ, enum_func, param).as_bool() {
            Ok(())
        } else {
            let error_code = GetLastError().0;
            Err(Error::new(error_code, build_error_message(error_code)))
        }
    }
}

pub fn enum_resource_types(module: HINSTANCE, enum_func: ENUMRESTYPEPROCW, param: isize) -> Result<()> {
    unsafe {
        if EnumResourceTypesW(module, enum_func, param).as_bool() {
            Ok(())
        } else {
            let error_code = GetLastError().0;
            Err(Error::new(error_code, build_error_message(error_code)))
        }
    }
}

pub fn find_resource(module: HINSTANCE, name: ResourceName, typ: ResourceType) -> Result<HRSRC> {
    unsafe {
        let name = match name {
            ResourceName::Id(id) => mem::transmute::<usize, PCWSTR>(id as usize),
            ResourceName::String(data_str) => {
                let data_str = utf8_to_wide(data_str);
                // TODO: Need the vector to stay around for the pointer.
            }
        }
        let name = mem::transmute::<usize, PCWSTR>(name as usize);
        let typ = mem::transmute::<usize, PCWSTR>(typ as usize);
        let res_handle = FindResourceW(module, name, typ);
        if res_handle.is_invalid() {
            let error_code = GetLastError().0;
            Err(Error::new(error_code, build_error_message(error_code)))
        } else {
            Ok(res_handle)
        }
    }
}
