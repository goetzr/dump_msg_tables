use std::ffi::c_void;
use std::fmt;
use std::mem::{self, MaybeUninit};

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Diagnostics::Debug::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::System::Memory::*;

#[derive(Debug)]
pub struct Error {
    code: u32,
    msg: String,
}

impl Error {
    fn last_error() -> Self {
        let code = unsafe { GetLastError().0 };
        Error {
            code,
            msg: Error::build_error_message(code),
        }
    }

    fn from_win_error(err: windows::core::Error) -> Self {
        Error {
            code: err.code().0 as u32,
            msg: err.message().to_string(),
        }
    }

    fn build_error_message(code: u32) -> String {
        unsafe {
            let mut buf = MaybeUninit::<PWSTR>::uninit();
            let ret = FormatMessageW(
                FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM,
                None,
                code,
                0,
                mem::transmute::<*mut PWSTR, PWSTR>(buf.as_mut_ptr()),
                0,
                None,
            );
            match ret {
                0 => "<error message unavailable>".to_string(),
                _ => {
                    let buf = buf.assume_init();
                    let mut msg = wide_to_utf8(buf.0);
                    LocalFree(mem::transmute::<PWSTR, isize>(buf));

                    // Remove any trailing whitespace.
                    let ws_len = msg.chars()
                        .rev()
                        .take_while(|&c| char::is_whitespace(c))
                        .count();
                    msg.truncate(msg.len() - ws_len);
                    msg
                }
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}) {}", self.code, self.msg)
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

#[inline]
fn wide_to_utf8(mut data: *const u16) -> String {
    let mut out = String::new();
    unsafe {
        while *data != 0 {
            out.push(char::from_u32_unchecked(*data as u32));
            data = data.add(1);
        }
    }
    out
}

#[inline]
fn ansi_to_utf8(mut data: *const u8) -> String {
    let mut out = String::new();
    unsafe {
        while *data != 0 {
            out.push(char::from_u32_unchecked(*data as u32));
            data = data.add(1);
        }
    }
    out
}

#[inline]
fn utf8_to_wide(data: &str) -> Vec<u16> {
    let mut out: Vec<u16> = data.encode_utf16().collect();
    // Add NULL terminator.
    out.push(0);
    out
}

#[inline]
fn clone_wide(mut data: *const u16) -> Vec<u16> {
    let mut out = Vec::new();
    unsafe {
        while *data != 0 {
            out.push(*data);
            data = data.add(1);
        }
    }
    // Add NULL terminator.
    out.push(0);
    out
}

pub enum ResourceId {
    Num(u16),
    String { wide: Vec<u16>,  utf8: String },
}

impl ResourceId {
    pub fn parse(data: PCWSTR) -> std::result::Result<Self, ()> {
        let data_num = unsafe { mem::transmute::<PCWSTR, usize>(data) };
        if data_num >> 16 == 0 {
            let num = (data_num & 0xffff) as u16;
            Ok(ResourceId::Num(num))
        } else {
            let data_str = wide_to_utf8(data.0);
            if data_str.starts_with("#") {
                let num = data_str[1..].parse::<u16>();
                match num {
                    Ok(num) => Ok(ResourceId::Num(num)),
                    Err(_) => Err(()),
                }
            } else {
                let wide = clone_wide(data.0);
                Ok(ResourceId::String {
                    wide,
                    utf8: data_str,
                })
            }
        }
    }

    pub fn pack(&self) -> PCWSTR {
        match &self {
            ResourceName::Num(num) => unsafe { mem::transmute::<usize, PCWSTR>(*num as usize) },
            ResourceName::String { wide, .. } => PCWSTR::from_raw(wide.as_ptr()),
        }
    }

    pub fn from_num(num: u16) -> Self {
        ResourceId::Num(num)
    }
}

impl ToString for ResourceId {
    fn to_string(&self) -> String {
        match &self {
            ResourceId::Num(num) => format!("{}", num),
            ResourceId::String { utf8, .. } => format!("{}", utf8),
        }
    }
}

pub type ResourceName = ResourceId;
pub type ResourceType = ResourceId;

pub fn load_library(mod_name: &str) -> Result<HINSTANCE> {
    let mod_name = utf8_to_wide(mod_name);
    let mod_name = PCWSTR(mod_name.as_ptr());
    unsafe { LoadLibraryW(mod_name).map_err(|e| Error::from_win_error(e)) }
}

pub const RT_MESSAGETABLE: u16 = 11;

pub fn enum_resource_names(
    module: HINSTANCE,
    typ: ResourceType,
    enum_func: ENUMRESNAMEPROCW,
    param: isize,
) -> Result<()> {
    if unsafe { EnumResourceNamesW(module, typ.pack(), enum_func, param).as_bool() } {
        Ok(())
    } else {
        Err(Error::last_error())
    }
}

pub fn find_resource(module: HINSTANCE, name: ResourceName, typ: ResourceType) -> Result<HRSRC> {
    let resource = unsafe { FindResourceW(module, name.pack(), typ.pack()) };
    if resource.is_invalid() {
        Err(Error::last_error())
    } else {
        Ok(resource)
    }
}

pub fn load_resource(module: HINSTANCE, resource: HRSRC) -> Result<isize> {
    let res_data = unsafe { LoadResource(module, resource) };
    if res_data == 0 {
        Err(Error::last_error())
    } else {
        Ok(res_data)
    }
}

pub fn lock_resource(res_data: isize) -> Result<*mut c_void> {
    let res_mem = unsafe { LockResource(res_data) };
    if res_mem.is_null() {
        Err(Error::last_error())
    } else {
        Ok(res_mem)
    }
}