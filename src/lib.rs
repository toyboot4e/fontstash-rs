/*!
Wrapper of [fontstash] (actually a [fork] of it)

[fontstash]: https://github.com/memononen/fontstash
[fork]: https://github.com/toyboot4e/fontstash-rs-src

# Custom renderer

`fontstash` is render-agnonstic. You can pull [`FonsQuad`]s via [`FonsTextIter`] and batch them to
make draw calls.

# Multiple lines of text

`fontstash-rs` doesn't handle multiple lines of text out of the box. You would need some layer to
draw or measure them.

# TODOs

* support state push/pop
*/

#![allow(unused_variables)]

pub use fontstash_sys as sys;

pub type Result<T> = std::result::Result<T, FonsError>;

use std::fmt;

/// Quadliteral
///
/// * `s0`, `t0`: left-up coner in the font texture
/// * `s1`, `t1`: right-down coner in the font texture
/// * `x0`, `y0`: left-up coner of the target position
/// * `x1`, `y1`: right-down coner of the target position
#[doc(inline)]
pub type FonsQuad = sys::FONSquad;

use std::os::raw::{c_int, c_uchar, c_void};

#[derive(Debug, Clone)]
pub enum FonsError {
    FailedToAllocFont(),
    FoundNoFont(),
    // `renderResize` returned `1`
    RenderResizeError(),
}

impl fmt::Display for FonsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::FailedToAllocFont() => {
                write!(f, "FontStash failed to allocate font")
            }
            Self::FoundNoFont() => {
                write!(f, "FontStash was unable to find font")
            }
            Self::RenderResizeError() => {
                write!(f, "FontStash detected `renderResize` returned `1`")
            }
        }
    }
}

/// Error code supplied to [`ErrorCallback`]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ErrorCode {
    ScratchFull = sys::FONSerrorCode_FONS_SCRATCH_FULL as u8,
    StatesOverflow = sys::FONSerrorCode_FONS_STATES_OVERFLOW as u8,
    StatesUnderflow = sys::FONSerrorCode_FONS_STATES_UNDERFLOW as u8,
}

impl ErrorCode {
    pub fn from_u32(x: u32) -> Option<Self> {
        Some(match x {
            sys::FONSerrorCode_FONS_SCRATCH_FULL => ErrorCode::ScratchFull,
            sys::FONSerrorCode_FONS_STATES_OVERFLOW => ErrorCode::StatesOverflow,
            sys::FONSerrorCode_FONS_STATES_UNDERFLOW => ErrorCode::StatesUnderflow,
            _ => return None,
        })
    }
}

/// The `error` argument is actually [`ErrorCode`]
pub type ErrorCallback = unsafe extern "C" fn(uptr: *mut c_void, error: c_int, val: c_int);

pub fn set_error_callback(
    raw_stash: *mut sys::FONScontext,
    callback: ErrorCallback,
    uptr: *mut c_void,
) {
    unsafe {
        sys::fonsSetErrorCallback(raw_stash, Some(callback), uptr);
    }
}

/// Set of callbacks
///
/// * `uptr`: user data pointer, which is usually the implementation of [`Renderer`]
///
/// Return non-zero to represent success.
pub unsafe trait Renderer {
    /// Creates font texture
    unsafe extern "C" fn create(uptr: *mut c_void, width: c_int, height: c_int) -> c_int;

    /// Create new texture
    ///
    /// User of [`Renderer`] should not call it directly; it's used to implement
    /// `FontStash::expand_atlas` and `FontStash::reset_atlas`.
    unsafe extern "C" fn resize(uptr: *mut c_void, width: c_int, height: c_int) -> c_int;

    /// Try to expand texture while the atlas is full
    unsafe extern "C" fn expand(uptr: *mut c_void) -> c_int;

    /// Update texture
    unsafe extern "C" fn update(uptr: *mut c_void, rect: *mut c_int, data: *const c_uchar)
        -> c_int;
}

#[derive(Debug)]
struct FonsContextDrop {
    raw: *mut sys::FONScontext,
}

impl Drop for FonsContextDrop {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                sys::fonsDeleteInternal(self.raw);
            }
        }
    }
}

/// Shared ownership of font stash
///
/// # Hack for creation
///
/// [`Renderer`] needs fixed memory position so that `fontstash::sys` can call callback methods
/// of it.
///
/// 1. Create [`Renderer`] in a `Box` with `FontStash` being `uninitialized`
/// 2. Call [`FontStash::init_mut`] to initialize `FontStash`
#[derive(Debug)]
pub struct FontStash {
    fons: std::rc::Rc<FonsContextDrop>,
}

/// Hack for creation
impl FontStash {
    pub fn uninitialized() -> Self {
        FontStash {
            fons: std::rc::Rc::new(FonsContextDrop {
                raw: std::ptr::null_mut(),
            }),
        }
    }

    pub fn init_mut<R: Renderer>(&mut self, w: u32, h: u32, renderer: *mut R) {
        self.fons = std::rc::Rc::new(Self::create(w, h, renderer));
    }
}

impl FontStash {
    pub fn raw(&self) -> *mut sys::FONScontext {
        self.fons.raw
    }

    pub fn clone(&self) -> Self {
        FontStash {
            fons: self.fons.clone(),
        }
    }

    /// Creates `FONScontext`
    ///
    /// The `renderer` has to have consistant memory position. Maybe put in in a `Box`.
    fn create<R: Renderer>(w: u32, h: u32, renderer: *mut R) -> FonsContextDrop {
        let flags = Flags::TopLeft;
        let params = sys::FONSparams {
            width: w as c_int,
            height: h as c_int,
            flags: flags as u8,
            userPtr: renderer as *mut _,
            renderCreate: Some(R::create),
            renderResize: Some(R::resize),
            renderExpand: Some(R::expand),
            renderUpdate: Some(R::update),
            // called before deleting font data but probablly we don't need it
            renderDelete: None,
        };

        FonsContextDrop {
            raw: unsafe { sys::fonsCreateInternal(&params as *const _ as *mut _) },
        }
    }
}

/// Font index
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontIx(u32);

impl FontIx {
    pub fn raw(&self) -> u32 {
        self.0
    }

    pub unsafe fn from_raw(ix: u32) -> Self {
        Self(ix)
    }
}

/// Font storage. Each font is keyed with `name` string.
impl FontStash {
    pub fn add_font_mem(&self, name: &str, data: &[u8]) -> Result<FontIx> {
        let name = std::ffi::CString::new(name).unwrap();

        let ix = unsafe {
            sys::fonsAddFontMem(
                self.raw(),
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
    //         base: c_int,
    //         fallback: c_int,
    //     ) -> c_int;
    // }

    pub fn set_font(&self, font: FontIx) {
        unsafe {
            sys::fonsSetFont(self.raw(), font.0 as i32);
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
impl FontStash {
    pub fn atlas_size(&self) -> [u32; 2] {
        let [mut x, mut y] = [0, 0];
        unsafe {
            sys::fonsGetAtlasSize(self.raw(), &mut x, &mut y);
        }
        [x as u32, y as u32]
    }

    /// Creates fontstash atlas size copying the previous data
    pub fn expand_atlas(&self, w: u32, h: u32) -> Result<()> {
        if unsafe { sys::fonsExpandAtlas(self.raw(), w as i32, h as i32) } != 0 {
            Ok(())
        } else {
            Err(FonsError::RenderResizeError())
        }
    }

    /// Creates new fontstash atlas with size without copying the previous data
    pub fn reset_atlas(&self, w: u32, h: u32) -> Result<()> {
        unsafe {
            if sys::fonsResetAtlas(self.raw(), w as i32, h as i32) == 1 {
                Ok(())
            } else {
                Err(FonsError::RenderResizeError())
            }
        }
    }
}

/// States
impl FontStash {
    /// NOTE: DPI scaling?
    pub fn set_size(&self, size: f32) {
        unsafe {
            sys::fonsSetSize(self.raw(), size);
        }
    }

    pub fn set_color(&self, color: u32) {
        unsafe {
            sys::fonsSetColor(self.raw(), color);
        }
    }

    /// Horizontal space between characters?
    pub fn set_spacing(&self, spacing: f32) {
        unsafe {
            sys::fonsSetSpacing(self.raw(), spacing);
        }
    }

    pub fn set_blur(&self, blur: f32) {
        unsafe {
            sys::fonsSetBlur(self.raw(), blur);
        }
    }

    /// Sets alignment of quadliterals returned with [`FontStash::text_iter`]
    pub fn set_align(&self, align: Align) {
        unsafe {
            sys::fonsSetAlign(self.raw(), align.bits() as i32);
        }
    }
}

/// Texture
impl FontStash {
    /// NOTE: Pixel is in one byte (8 bits alpha channel only)
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

    // FIXME: what's this
    // pub fn dirty(&self) -> (bool, i32) {
    //     let mut dirty_flags = 0;
    //     let x = unsafe { sys::fonsValidateTexture(self.raw(), &mut dirty_flags) };
    //     (x == 1, dirty_flags)
    // }
}

/// State stack
impl FontStash {
    pub fn push_state(&mut self) {
        unsafe {
            sys::fonsPushState(self.raw());
        }
    }

    pub fn pop_state(&mut self) {
        unsafe {
            sys::fonsPopState(self.raw());
        }
    }

    pub fn clear_state(&mut self) {
        unsafe {
            sys::fonsClearState(self.raw());
        }
    }
}

/// Draw
impl FontStash {
    /// Iterator of quadliterals aligned with [`Align`]
    pub fn text_iter(&self, text: &str) -> Result<FonsTextIter> {
        FonsTextIter::from_text(self.clone(), text)
    }
}

/// Measure
impl FontStash {
    /// Returns `[left_x, top_y, right_x, bottom_y]`. Note that **it doesn't handle multiple lines
    /// of text**. You need custom layer for multi-line text!
    pub fn text_bounds_oneline(&self, pos: [f32; 2], text: &str) -> [f32; 4] {
        let mut bounds = [0.0; 4];

        // why does fontstash return width..
        let _width = unsafe {
            let start = text.as_ptr() as *const _;
            let end = text.as_ptr().add(text.len()) as *const _;
            sys::fonsTextBounds(self.raw(), pos[0], pos[1], start, end, bounds.as_mut_ptr())
        };

        bounds
    }

    /// Returns `[width, height]`. Note that **it doesn't handle multiple lines of text**.
    /// You need custom layer for multi-line text!
    pub fn text_size_oneline(&self, text: &str) -> [f32; 2] {
        let mut bounds = [0.0; 4];

        // why does fontstash return width..
        let _width = unsafe {
            let start = text.as_ptr() as *const _;
            let end = text.as_ptr().add(text.len()) as *const _;
            sys::fonsTextBounds(self.raw(), 0.0, 0.0, start, end, bounds.as_mut_ptr())
        };

        let w = bounds[2] - bounds[0];
        let h = bounds[3] - bounds[1];
        [w, h]
    }

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

bitflags::bitflags! {
    /// Alignment of each quadliteral returned by [`FontStash::text_iter`]
    pub struct Align: u32 {
        const BASELINE = sys::FONSalign_FONS_ALIGN_BASELINE;
        const BOTTOM = sys::FONSalign_FONS_ALIGN_BOTTOM;
        const CENTER = sys::FONSalign_FONS_ALIGN_CENTER;
        const LEFT = sys::FONSalign_FONS_ALIGN_LEFT;
        const MID = sys::FONSalign_FONS_ALIGN_MIDDLE;
        const RIGHT = sys::FONSalign_FONS_ALIGN_RIGHT;
        const TOP = sys::FONSalign_FONS_ALIGN_TOP;
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Flags {
    BottomLeft = sys::FONSflags_FONS_ZERO_BOTTOMLEFT as u8,
    TopLeft = sys::FONSflags_FONS_ZERO_TOPLEFT as u8,
}

/// Iterator of text quads
pub struct FonsTextIter {
    stash: FontStash,
    iter: sys::FONStextIter,
    is_running: bool,
}

impl FonsTextIter {
    pub fn from_text(stash: FontStash, text: &str) -> Result<Self> {
        unsafe {
            // `FONStextIter` iterates through [start, end)
            let start = text.as_ptr() as *const _;
            let end = text.as_ptr().add(text.len()) as *const _;

            let mut iter: sys::FONStextIter = std::mem::zeroed();
            let res = sys::fonsTextIterInit(stash.raw(), &mut iter as *mut _, 0.0, 0.0, start, end);

            if res == 0 {
                // failed
                return Err(FonsError::FoundNoFont());
            }

            Ok(Self {
                stash: stash.clone(),
                iter,
                is_running: res == 1,
            })
        }
    }
}

impl Iterator for FonsTextIter {
    type Item = FonsQuad;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.is_running {
            return None;
        }

        let mut quad = unsafe { std::mem::zeroed() };

        let res = unsafe {
            sys::fonsTextIterNext(
                self.stash.raw(),
                &mut self.iter as *mut _,
                &mut quad as *mut _,
            )
        };

        if res == 1 {
            // continue
            Some(quad)
        } else {
            // end
            None
        }
    }
}
