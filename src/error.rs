use std::fmt;
use std::mem::{self, MaybeUninit};

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Diagnostics::Debug::*;
use windows::Win32::System::Memory::*;

use crate::str_util;

#[derive(Debug)]
pub struct Error {
    code: u32,
    msg: String,
}

impl Error {
    pub fn last_error() -> Self {
        let code = unsafe { GetLastError().0 };
        Error {
            code,
            msg: Error::build_error_message(code),
        }
    }

    pub fn from_win_error(err: windows::core::Error) -> Self {
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
                    let mut msg = str_util::utf16_to_utf8(buf.0);
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

pub type Result<T> = std::result::Result<T, Error>;