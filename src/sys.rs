use std::ffi::c_void;
use std::mem;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::*;

use crate::str_util;
use crate::error;

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
            let data_str = str_util::utf16_to_utf8(data.0);
            if data_str.starts_with("#") {
                let num = data_str[1..].parse::<u16>();
                match num {
                    Ok(num) => Ok(ResourceId::Num(num)),
                    Err(_) => Err(()),
                }
            } else {
                let wide = str_util::clone_utf16(data.0);
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

pub fn load_library(mod_name: &str) -> error::Result<HINSTANCE> {
    let mod_name = str_util::utf8_to_utf16(mod_name);
    let mod_name = PCWSTR(mod_name.as_ptr());
    unsafe { LoadLibraryW(mod_name).map_err(|e| error::Error::from_win_error(e)) }
}

pub const RT_MESSAGETABLE: u16 = 11;

pub fn enum_resource_names(
    module: HINSTANCE,
    typ: ResourceType,
    enum_func: ENUMRESNAMEPROCW,
    param: isize,
) -> error::Result<()> {
    if unsafe { EnumResourceNamesW(module, typ.pack(), enum_func, param).as_bool() } {
        Ok(())
    } else {
        Err(error::Error::last_error())
    }
}

pub fn find_resource(module: HINSTANCE, name: ResourceName, typ: ResourceType) -> error::Result<HRSRC> {
    let resource = unsafe { FindResourceW(module, name.pack(), typ.pack()) };
    if resource.is_invalid() {
        Err(error::Error::last_error())
    } else {
        Ok(resource)
    }
}

pub fn load_resource(module: HINSTANCE, resource: HRSRC) -> error::Result<isize> {
    let res_data = unsafe { LoadResource(module, resource) };
    if res_data == 0 {
        Err(error::Error::last_error())
    } else {
        Ok(res_data)
    }
}

pub fn lock_resource(res_data: isize) -> error::Result<*mut c_void> {
    let res_mem = unsafe { LockResource(res_data) };
    if res_mem.is_null() {
        Err(error::Error::last_error())
    } else {
        Ok(res_mem)
    }
}