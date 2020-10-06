//! Wrapper of [fontstash]
//!
//! [fontstash]: https://github.com/memononen/fontstash
//!
//! # Custom renderer
//!
//! `fontstash-rs` doesn't include the edfault renderer in the original repository.
//!
//! There are two ways to draw texts:
//!
//! ## Iterator ([`FonsTextIter`])
//!
//! You would batch quads manually.
//!
//! ## Callback
//!
//! The rendering callback method of [`Renderer`] is.. not so good. They're not provided as arrays
//! of vertex components. Prefer the iterator.
//!
//! # References
//!
//! * https://github.com/prime31/via/blob/master/fonts/fontbook.v

pub use fontstash_sys as sys;

/// Set of callbacks
///
/// * `uptr`: user pointer that can be casted to the implementation of `Renderer`
pub unsafe trait Renderer {
    /// Initialize resource here
    ///
    /// Return `1` to represent success.
    unsafe extern "C" fn create(
        uptr: *mut std::os::raw::c_void,
        width: std::os::raw::c_int,
        height: std::os::raw::c_int,
    ) -> std::os::raw::c_int;

    /// Recreation can be sufficient
    ///
    /// Return `1` to represent success.
    unsafe extern "C" fn resize(
        uptr: *mut std::os::raw::c_void,
        width: std::os::raw::c_int,
        height: std::os::raw::c_int,
    ) -> std::os::raw::c_int;

    unsafe extern "C" fn update(
        uptr: *mut std::os::raw::c_void,
        rect: *mut std::os::raw::c_int,
        data: *const std::os::raw::c_uchar,
    );

    /// Make a draw call
    ///
    /// This is a callback approach via [`draw_text`]. However, it's not recommended; you would
    /// have to convert the array of structs (positions, texels and colors) into array of vertices
    /// after all. So I recommend using [`FonsTextIter`].
    unsafe extern "C" fn draw(
        uptr: *mut std::os::raw::c_void,
        verts: *const f32,
        tcoords: *const f32,
        colors: *const std::os::raw::c_uint,
        nverts: std::os::raw::c_int,
    ) {
    }

    /// Free user texture data here
    unsafe extern "C" fn delete(uptr: *mut std::os::raw::c_void);
}

// --------------------------------------------------------------------------------
// Owned version of `FonsContext`

pub struct FonsContextDrop {
    pub fons: FonsContext,
}

impl std::ops::Deref for FonsContextDrop {
    type Target = FonsContext;
    fn deref(&self) -> &Self::Target {
        &self.fons
    }
}

impl FonsContextDrop {
    pub fn create<R: Renderer>(w: u32, h: u32, renderer: *mut R) -> Self {
        Self {
            fons: FonsContext::create(w, h, renderer),
        }
    }

    /// Hack to create [`FonsContextDrop`] (set it with [`create`] soon)
    pub fn null() -> Self {
        Self {
            fons: FonsContext {
                raw: std::ptr::null_mut(),
            },
        }
    }
}

pub fn delete(cx: *mut sys::FONScontext) {
    unsafe {
        sys::fonsDeleteInternal(cx);
    }
}

impl Drop for FonsContextDrop {
    fn drop(&mut self) {
        self::delete(self.fons.raw);
    }
}

// --------------------------------------------------------------------------------
// `FonsContext`, smarter pointer of [`FONScontext`]

/// Smarter pointer of [`sys::FONScontext`] with methods
#[derive(Debug, Clone, Copy)]
pub struct FonsContext {
    pub raw: *mut sys::FONScontext,
}

impl FonsContext {
    /// Creates `FONScontext`
    ///
    /// The `renderer` has to have consistant immory position. Maybe put in in a `Box`.
    pub fn create<R: Renderer>(w: u32, h: u32, renderer: *mut R) -> Self {
        let params = sys::FONSparams {
            width: w as std::os::raw::c_int,
            height: h as std::os::raw::c_int,
            flags: Flags::TopLeft as u8,
            userPtr: renderer as *mut _,
            renderCreate: Some(R::create),
            renderResize: Some(R::resize),
            renderUpdate: Some(R::update),
            renderDraw: Some(R::draw),
            renderDelete: Some(R::delete),
        };

        Self {
            raw: unsafe { sys::fonsCreateInternal(&params as *const _ as *mut _) },
        }
    }

    pub fn add_font_mem(&self, name: &str, data: &[u8]) {
        let name = std::ffi::CString::new(name).unwrap();
        unsafe {
            sys::fonsAddFontMem(
                self.raw,
                name.as_ptr() as *const _,
                data as *const _ as *mut _,
                data.len() as i32,
                false as i32,
            );
        }
    }

    pub fn set_size(&self, size: f32) {
        unsafe {
            sys::fonsSetSize(self.raw, size);
        }
    }

    pub fn set_color(&self, color: u32) {
        unsafe {
            sys::fonsSetColor(self.raw, color);
        }
    }

    pub fn text_iter(&self, text: &str) -> FonsTextIter {
        FonsTextIter::new(self, text)
    }
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

// extern "C" {
//     pub fn fonsSetErrorCallback(
//         s: *mut FONScontext,
//         callback: ::std::option::Option<
//             unsafe extern "C" fn(
//                 uptr: *mut ::std::os::raw::c_void,
//                 error: ::std::os::raw::c_int,
//                 val: ::std::os::raw::c_int,
//             ),
//         >,
//         uptr: *mut ::std::os::raw::c_void,
//     );
// }

// extern "C" {
//     pub fn fonsGetAtlasSize(
//         s: *mut FONScontext,
//         width: *mut ::std::os::raw::c_int,
//         height: *mut ::std::os::raw::c_int,
//     );
// }

// extern "C" {
//     pub fn fonsExpandAtlas(
//         s: *mut FONScontext,
//         width: ::std::os::raw::c_int,
//         height: ::std::os::raw::c_int,
//     ) -> ::std::os::raw::c_int;
// }

// extern "C" {
//     pub fn fonsResetAtlas(
//         stash: *mut FONScontext,
//         width: ::std::os::raw::c_int,
//         height: ::std::os::raw::c_int,
//     ) -> ::std::os::raw::c_int;
// }

// extern "C" {
//     pub fn fonsAddFont(
//         s: *mut FONScontext,
//         name: *const ::std::os::raw::c_char,
//         path: *const ::std::os::raw::c_char,
//     ) -> ::std::os::raw::c_int;
// }

// extern "C" {
//     pub fn fonsGetFontByName(
//         s: *mut FONScontext,
//         name: *const ::std::os::raw::c_char,
//     ) -> ::std::os::raw::c_int;
// }

// extern "C" {
//     pub fn fonsAddFallbackFont(
//         stash: *mut FONScontext,
//         base: ::std::os::raw::c_int,
//         fallback: ::std::os::raw::c_int,
//     ) -> ::std::os::raw::c_int;
// }

// extern "C" {
//     pub fn fonsPushState(s: *mut FONScontext);
// }

// extern "C" {
//     pub fn fonsPopState(s: *mut FONScontext);
// }

// extern "C" {
//     pub fn fonsClearState(s: *mut FONScontext);
// }

// extern "C" {
//     pub fn fonsSetSpacing(s: *mut FONScontext, spacing: f32);
// }

// extern "C" {
//     pub fn fonsSetBlur(s: *mut FONScontext, blur: f32);
// }

// extern "C" {
//     pub fn fonsSetAlign(s: *mut FONScontext, align: ::std::os::raw::c_int);
// }

// extern "C" {
//     pub fn fonsSetFont(s: *mut FONScontext, font: ::std::os::raw::c_int);
// }

// extern "C" {
//     pub fn fonsDrawText(
//         s: *mut FONScontext,
//         x: f32,
//         y: f32,
//         string: *const ::std::os::raw::c_char,
//         end: *const ::std::os::raw::c_char,
//     ) -> f32;
// }

// extern "C" {
//     pub fn fonsTextBounds(
//         s: *mut FONScontext,
//         x: f32,
//         y: f32,
//         string: *const ::std::os::raw::c_char,
//         end: *const ::std::os::raw::c_char,
//         bounds: *mut f32,
//     ) -> f32;
// }

// extern "C" {
//     pub fn fonsLineBounds(s: *mut FONScontext, y: f32, miny: *mut f32, maxy: *mut f32);
// }

// extern "C" {
//     pub fn fonsVertMetrics(
//         s: *mut FONScontext,
//         ascender: *mut f32,
//         descender: *mut f32,
//         lineh: *mut f32,
//     );
// }

// extern "C" {
//     pub fn fonsGetTextureData(
//         stash: *mut FONScontext,
//         width: *mut ::std::os::raw::c_int,
//         height: *mut ::std::os::raw::c_int,
//     ) -> *const ::std::os::raw::c_uchar;
// }

// extern "C" {
//     pub fn fonsValidateTexture(
//         s: *mut FONScontext,
//         dirty: *mut ::std::os::raw::c_int,
//     ) -> ::std::os::raw::c_int;
// }

// extern "C" {
//     pub fn fonsDrawDebug(s: *mut FONScontext, x: f32, y: f32);
// }

/// Iterator of text used with `while` loop
pub struct FonsTextIter {
    stash: FonsContext,
    iter: sys::FONStextIter,
    is_running: bool,
    quad: sys::FONSquad,
}

impl FonsTextIter {
    pub fn new(stash: &FonsContext, text: &str) -> Self {
        unsafe {
            let start = text.as_ptr() as *const _;
            let end = text.as_ptr().add(text.len()) as *const _;

            let mut iter: sys::FONStextIter = std::mem::zeroed();
            let res = sys::fonsTextIterInit(stash.raw, &mut iter as *mut _, 0.0, 0.0, start, end);

            let quad = std::mem::zeroed();

            Self {
                stash: stash.clone(),
                iter,
                quad,
                is_running: res == 1,
                // _marker: std::marker::PhantomData,
            }
        }
    }

    pub fn next(&mut self) -> Option<&mut sys::FONSquad> {
        if !self.is_running {
            return None;
        }

        let res = unsafe {
            sys::fonsTextIterNext(
                self.stash.raw,
                &mut self.iter as *mut _,
                &mut self.quad as *mut _,
            )
        };

        if res == 1 {
            Some(&mut self.quad)
        } else {
            None
        }
    }
}
