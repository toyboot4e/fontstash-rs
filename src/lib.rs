//! Wrapper of [fontstash]
//!
//! [fontstash]: https://github.com/memononen/fontstash
//!
//! # Custom renderer
//!
//! `fontstash-rs` doesn't include the edfault renderer in the original repository.
//!
//! c.f. https://github.com/prime31/via/blob/master/fonts/fontbook.v

use std::{
    ffi::c_void,
    os::raw::{c_int, c_uchar, c_uint},
};

pub use fontstash_sys as sys;
pub type FonsContext = sys::FONScontext;

pub fn create<R: Renderer>(w: u32, h: u32, renderer: &R) -> *mut sys::FONScontext {
    let params = sys::FONSparams {
        width: w as c_int,
        height: h as c_int,
        flags: Flags::TopLeft as u8,
        userPtr: renderer as *const _ as *mut _,
        renderCreate: Some(R::create),
        renderResize: Some(R::resize),
        renderUpdate: Some(R::update),
        renderDraw: Some(R::draw),
        renderDelete: Some(R::delete),
    };
    unsafe { sys::fonsCreateInternal(&params as *const _ as *mut _) }
}

pub fn delete(cx: *mut sys::FONScontext) {
    unsafe {
        sys::fonsDeleteInternal(cx);
    }
}

/// Set of callbacks
///
/// * `uptr`: user pointer that can be casted to the implementation of `Renderer`
pub unsafe trait Renderer {
    /// Return `1` to represent success
    unsafe extern "C" fn create(
        uptr: *mut ::std::os::raw::c_void,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
    /// Return `1` to represent success. Recreation can be sufficient
    unsafe extern "C" fn resize(uptr: *mut c_void, width: c_int, height: c_int) -> c_int;
    unsafe extern "C" fn update(uptr: *mut c_void, rect: *mut c_int, data: *const c_uchar);
    unsafe extern "C" fn draw(
        uptr: *mut c_void,
        verts: *const f32,
        tcoords: *const f32,
        colors: *const c_uint,
        nverts: c_int,
    );
    /// Free user texture data here
    unsafe extern "C" fn delete(uptr: *mut c_void);
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Align {
    BaseLin = sys::FONSalign_FONS_ALIGN_BASELINE as u8,
    Bottom = sys::FONSalign_FONS_ALIGN_BOTTOM as u8,
    Center = sys::FONSalign_FONS_ALIGN_CENTER as u8,
    Left = sys::FONSalign_FONS_ALIGN_LEFT as u8,
    Mid = sys::FONSalign_FONS_ALIGN_MIDDLE as u8,
    Right = sys::FONSalign_FONS_ALIGN_RIGHT as u8,
    Top = sys::FONSalign_FONS_ALIGN_TOP as u8,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ErrorCode {
    AtlasFull = sys::FONSerrorCode_FONS_ATLAS_FULL as u8,
    ScratchFull = sys::FONSerrorCode_FONS_SCRATCH_FULL as u8,
    StatesOverflow = sys::FONSerrorCode_FONS_STATES_OVERFLOW as u8,
    StatesUnderflow = sys::FONSerrorCode_FONS_STATES_UNDERFLOW as u8,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Flags {
    BottomLeft = sys::FONSflags_FONS_ZERO_BOTTOMLEFT as u8,
    TopLeft = sys::FONSflags_FONS_ZERO_TOPLEFT as u8,
}

