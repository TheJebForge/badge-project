use std::ffi::{CString, NulError};
use std::fmt::Display;
use std::os::raw::c_char;
use std::str::FromStr;
use egui::RichText;

pub fn string_to_char_array <const N: usize> (input: &str) -> Result<[c_char; N], NulError> {
    let cstr = CString::from_str(if input.len() > N - 1 {
        &input[0..N - 1]
    } else {
        input
    })?;

    let mut char_array = [0 as c_char; N];

    let mut bytes = cstr.as_bytes_with_nul().iter().map(|c| *c as c_char);
    char_array.fill_with(|| bytes.next().unwrap_or(0));

    Ok(char_array)
}

pub unsafe fn any_as_u8_vec<T: Sized>(p: &T) -> Vec<u8> { 
    unsafe {
        core::slice::from_raw_parts(
            (p as *const T) as *const u8,
            size_of::<T>(),
        )
    }.to_vec()
}

pub trait TuplePick<T> {
    fn pick_min(&self) -> T;
    fn pick_max(&self) -> T;
}

impl<T: Ord + Copy> TuplePick<T> for (T, T) {
    fn pick_min(&self) -> T {
        T::min(self.0, self.1)
    }

    fn pick_max(&self) -> T {
        T::max(self.0, self.1)
    }
}

pub trait AsRichText {
    fn rich(&self) -> RichText;
}

impl<T: Display> AsRichText for T {
    fn rich(&self) -> RichText {
        RichText::new(self.to_string())
    }
}