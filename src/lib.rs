//! Wrapper of [fontstash]
//!
//! [fontstash]: https://github.com/memononen/fontstash
//!
//! # Custom renderer
//!
//! `fontstash-rs` doesn't include the default renderer in the original repository. You have to
//! write your own.
//!
//! You pull [`FONSquad`](crate::sys::FONSquad]s via [`FonsTextIter`] and batch them to make draw
//! calls.
//!
//! The callback-based renderer is excluded from this crate.
//!
//! # References
//!
//! * https://github.com/prime31/via/blob/master/fonts/fontbook.v

#![allow(unused_variables)]

pub use ::fontstash_sys as sys;

pub type Result<T> = ::std::result::Result<T, FonsError>;

#[derive(Debug, Clone)]
pub enum FonsError {
    FailedToAllocFont(),
    FoundNoFont(),
    /// `renderResize` returned `1`
    RenderResizeError(),
}

/// Error code supplied to [`ErrorCallBack`]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ErrorCode {
    AtlasFull = sys::FONSerrorCode_FONS_ATLAS_FULL as u8,
    ScratchFull = sys::FONSerrorCode_FONS_SCRATCH_FULL as u8,
    StatesOverflow = sys::FONSerrorCode_FONS_STATES_OVERFLOW as u8,
    StatesUnderflow = sys::FONSerrorCode_FONS_STATES_UNDERFLOW as u8,
}

impl ErrorCode {
    pub fn from_u32(x: u32) -> Option<Self> {
        Some(match x {
            sys::FONSerrorCode_FONS_ATLAS_FULL => ErrorCode::AtlasFull,
            sys::FONSerrorCode_FONS_SCRATCH_FULL => ErrorCode::ScratchFull,
            sys::FONSerrorCode_FONS_STATES_OVERFLOW => ErrorCode::StatesOverflow,
            sys::FONSerrorCode_FONS_STATES_UNDERFLOW => ErrorCode::StatesUnderflow,
            _ => return None,
        })
    }
}

/// The [`error`] is actually [`ErrorCode`]
pub type ErrorCallback = unsafe extern "C" fn(
    uptr: *mut ::std::os::raw::c_void,
    error: ::std::os::raw::c_int,
    val: ::std::os::raw::c_int,
);

pub fn set_error_callback(
    raw_stash: *mut sys::FONScontext,
    callback: ErrorCallback,
    uptr: *mut ::std::os::raw::c_void,
) {
    unsafe {
        sys::fonsSetErrorCallback(raw_stash, Some(callback), uptr);
    }
}

/// Set of callbacks
///
/// * `uptr`: user data pointer, which is usually the implementation of [`Renderer`
pub unsafe trait Renderer {
    /// Creates font texture
    ///
    /// Return non-zero to represent success.
    unsafe extern "C" fn create(
        uptr: *mut std::os::raw::c_void,
        width: std::os::raw::c_int,
        height: std::os::raw::c_int,
    ) -> std::os::raw::c_int;

    /// Free user texture data here
    unsafe extern "C" fn delete(uptr: *mut std::os::raw::c_void);

    /// Create new texture
    ///
    /// Return non-zero to represent success.
    unsafe extern "C" fn resize(
        uptr: *mut std::os::raw::c_void,
        width: std::os::raw::c_int,
        height: std::os::raw::c_int,
    ) -> std::os::raw::c_int;

    /// Create new texture
    ///
    /// Return `1` to represent success; it's actually boolean.
    unsafe extern "C" fn update(
        uptr: *mut std::os::raw::c_void,
        rect: *mut std::os::raw::c_int,
        data: *const std::os::raw::c_uchar,
    ) -> std::os::raw::c_int;
}

/// Reference counted version of [`FonsContextDrop`]
pub struct FonsContext {
    fons: std::rc::Rc<FonsContextDrop>,
}

impl std::ops::Deref for FonsContext {
    type Target = FonsContextDrop;
    fn deref(&self) -> &Self::Target {
        self.fons.as_ref()
    }
}

impl FonsContext {
    pub fn raw(&self) -> *mut sys::FONScontext {
        self.fons.as_ref() as *const _ as *mut _
    }

    /// The owner of `FonsContext` needs to have fixed memory location, so it's often stored in
    /// [`Box`]. So first create the owner with uninitialized [`FonsContext`] and then initialize
    /// it.
    pub fn uninitialized() -> Self {
        Self {
            fons: std::rc::Rc::new(FonsContextDrop {
                raw: std::ptr::null_mut(),
            }),
        }
    }

    pub fn init_mut<R: Renderer>(&mut self, w: u32, h: u32, renderer: *mut R) {
        self.fons = std::rc::Rc::new(FonsContextDrop::create(w, h, renderer));
    }

    pub fn clone(&self) -> Self {
        FonsContext {
            fons: self.fons.clone(),
        }
    }
}

/// Wrapped version of [`sys::FONScontext`] with methods
#[derive(Debug)]
pub struct FonsContextDrop {
    raw: *mut sys::FONScontext,
}

impl Drop for FonsContextDrop {
    fn drop(&mut self) {
        unsafe {
            sys::fonsDeleteInternal(self.raw);
        }
    }
}

impl FonsContextDrop {
    pub fn raw(&self) -> *mut sys::FONScontext {
        self.raw
    }

    /// Creates `FONScontext`
    ///
    /// The `renderer` has to have consistant memory position. Maybe put in in a `Box`.
    pub fn create<R: Renderer>(w: u32, h: u32, renderer: *mut R) -> Self {
        let flags = Flags::TopLeft;
        let params = sys::FONSparams {
            width: w as std::os::raw::c_int,
            height: h as std::os::raw::c_int,
            flags: flags as u8,
            userPtr: renderer as *mut _,
            renderCreate: Some(R::create),
            renderResize: Some(R::resize),
            renderUpdate: Some(R::update),
            renderDelete: Some(R::delete),
        };

        FonsContextDrop {
            raw: unsafe { sys::fonsCreateInternal(&params as *const _ as *mut _) },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontIx(u32);

/// Font storage
impl FonsContextDrop {
    pub fn add_font_mem(&self, name: &str, data: &[u8]) -> Result<FontIx> {
        let name = std::ffi::CString::new(name).unwrap();

        let ix = unsafe {
            sys::fonsAddFontMem(
                self.raw,
                name.as_ptr() as *const _,
                data as *const _ as *mut _,
                data.len() as i32,
                false as i32,
            )
        };

        if ix == sys::FONS_INVALID {
            Err(FonsError::FailedToAllocFont())
        } else {
            Ok(FontIx(ix as u32))
        }
    }

    // extern "C" {
    //     pub fn fonsAddFallbackFont(
    //         stash: *mut FONScontext,
    //         base: ::std::os::raw::c_int,
    //         fallback: ::std::os::raw::c_int,
    //     ) -> ::std::os::raw::c_int;
    // }

    pub fn set_font(&self, font: FontIx) {
        unsafe {
            sys::fonsSetFont(self.raw, font.0 as i32);
        }
    }

    pub fn font_ix_by_name(&self, name: &str) -> Option<FontIx> {
        let name = std::ffi::CString::new(name).ok()?;
        let ix = unsafe { sys::fonsGetFontByName(self.raw(), name.as_ptr()) };
        if ix == sys::FONS_INVALID {
            None
        } else {
            Some(FontIx(ix as u32))
        }
    }
}

/// Atlas
impl FonsContextDrop {
    pub fn atlas_size(&self) -> [u32; 2] {
        let [mut x, mut y] = [0, 0];
        unsafe {
            sys::fonsGetAtlasSize(self.raw(), &mut x, &mut y);
        }
        [x as u32, y as u32]
    }

    /// Returns true if succeeded
    pub fn reset_atlas(&self, w: u32, h: u32) -> Result<()> {
        unsafe {
            if sys::fonsResetAtlas(self.raw(), w as i32, h as i32) == 1 {
                Ok(())
            } else {
                Err(FonsError::RenderResizeError())
            }
        }
    }

    /// Returns true if succeed
    pub fn expand_atlas(&self, w: u32, h: u32) -> Result<()> {
        if unsafe { sys::fonsExpandAtlas(self.raw(), w as i32, h as i32) } == 1 {
            Ok(())
        } else {
            Err(FonsError::RenderResizeError())
        }
    }
}

/// Font style state
impl FonsContextDrop {
    /// TODO: add DPI scaling factor to field
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

    // extern "C" {
    //     pub fn fonsSetSpacing(s: *mut FONScontext, spacing: f32);
    // }

    // extern "C" {
    //     pub fn fonsSetBlur(s: *mut FONScontext, blur: f32);
    // }

    // extern "C" {
    //     pub fn fonsSetAlign(s: *mut FONScontext, align: ::std::os::raw::c_int);
    // }
}

/// Texture
impl FonsContextDrop {
    /// Note that each pixel is in one byte (8 bits alpha channel only)
    pub fn with_pixels(&self, mut f: impl FnMut(&[u8], u32, u32)) {
        let (mut w, mut h) = (0, 0);
        let ptr = unsafe { sys::fonsGetTextureData(self.raw(), &mut w, &mut h) };
        if !ptr.is_null() {
            let pixels = unsafe { std::slice::from_raw_parts(ptr, (w * h) as usize) };
            f(pixels, w as u32, h as u32);
        } else {
            eprintln!("fontstash-rs: fonsGetTextureData returned null");
        }
    }

    /// FIXME: this
    pub fn dirty(&self) -> (bool, i32) {
        let mut dirty_flags = 0;
        let x = unsafe { sys::fonsValidateTexture(self.raw(), &mut dirty_flags) };
        (x == 1, dirty_flags)
    }
}

/// Draw
impl FonsContext {
    /// Iterator-based rendering
    pub fn text_iter(&self, text: &str) -> Result<FonsTextIter> {
        FonsTextIter::from_text(self.clone(), text)
    }
}

/// State stack
impl FonsContext {
    // extern "C" {
    //     pub fn fonsPushState(s: *mut FONScontext);
    // }

    // extern "C" {
    //     pub fn fonsPopState(s: *mut FONScontext);
    // }

    // extern "C" {
    //     pub fn fonsClearState(s: *mut FONScontext);
    // }
}

/// Measure
impl FonsContext {
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
pub enum Flags {
    BottomLeft = sys::FONSflags_FONS_ZERO_BOTTOMLEFT as u8,
    TopLeft = sys::FONSflags_FONS_ZERO_TOPLEFT as u8,
}

/// Iterator of text used with `while` loop
pub struct FonsTextIter {
    stash: FonsContext,
    iter: sys::FONStextIter,
    is_running: bool,
    quad: sys::FONSquad,
}

impl FonsTextIter {
    pub fn from_text(stash: FonsContext, text: &str) -> Result<Self> {
        unsafe {
            // `FONStextIter` iterates through [start, end
            let start = text.as_ptr() as *const _;
            let end = text.as_ptr().add(text.len()) as *const _;

            // the iterator is initialized with `stash->spacing`
            let mut iter: sys::FONStextIter = std::mem::zeroed();
            let res = sys::fonsTextIterInit(stash.raw, &mut iter as *mut _, 0.0, 0.0, start, end);

            if res == 0 {
                // failed
                return Err(FonsError::FoundNoFont());
            }

            let quad = std::mem::zeroed();

            Ok(Self {
                stash: stash.clone(),
                iter,
                quad,
                is_running: res == 1,
            })
        }
    }

    pub fn next(&mut self) -> Option<&sys::FONSquad> {
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
            // continue
            Some(&self.quad)
        } else {
            // end
            None
        }
    }
}
