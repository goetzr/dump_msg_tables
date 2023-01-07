#[inline]
pub fn utf16_to_utf8(mut data: *const u16) -> String {
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
pub fn ansi_to_utf8(mut data: *const u8) -> String {
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
pub fn utf8_to_utf16(data: &str) -> Vec<u16> {
    let mut out: Vec<u16> = data.encode_utf16().collect();
    // Add NULL terminator.
    out.push(0);
    out
}

#[inline]
pub fn clone_utf16(mut data: *const u16) -> Vec<u16> {
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