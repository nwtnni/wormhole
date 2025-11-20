use core::ffi;
use core::marker::PhantomData;

unsafe extern "C" {
    fn wh_create() -> *mut ffi::c_void;
    fn wh_destroy(wormhole: *mut ffi::c_void);

    unsafe fn wh_ref(wormhole: *mut ffi::c_void) -> *mut ffi::c_void;
    unsafe fn wh_unref(wormref: *mut ffi::c_void);

    unsafe fn wh_put(
        wormref: *mut ffi::c_void,
        kbuf: *const ffi::c_void,
        klen: u32,
        vbuf: *const ffi::c_void,
        vlen: u32,
    ) -> bool;

    unsafe fn wh_get(
        wormref: *const ffi::c_void,
        kbuf: *const ffi::c_void,
        klen: u32,
        vbuf_out: *mut ffi::c_void,
        vbuf_size: u32,
        vlen_out: *mut u32,
    ) -> bool;

    unsafe fn wh_del(wormref: *mut ffi::c_void, kbuf: *const ffi::c_void, klen: u32) -> bool;

    unsafe fn wh_iter_create(wormref: *mut ffi::c_void) -> *mut ffi::c_void;
    unsafe fn wh_iter_destroy(iter: *mut ffi::c_void);
    unsafe fn wh_iter_seek(iter: *mut ffi::c_void, kbuf: *const ffi::c_void, klen: u32);
    unsafe fn wh_iter_valid(iter: *mut ffi::c_void) -> bool;
    unsafe fn wh_iter_peek(
        iter: *mut ffi::c_void,
        kbuf_out: *mut ffi::c_void,
        kbuf_size: u32,
        klen_out: *mut u32,
        vbuf_out: *mut ffi::c_void,
        vbuf_size: u32,
        vlen_out: *mut u32,
    ) -> bool;
    unsafe fn wh_iter_skip1(iter: *mut ffi::c_void) -> bool;
}

pub struct Wormhole(*mut ffi::c_void);

impl Wormhole {
    pub fn pin(&self) -> WormRef<'_> {
        WormRef {
            inner: unsafe { wh_ref(self.0) },
            _wormhole: PhantomData,
        }
    }
}

unsafe impl Send for Wormhole {}
unsafe impl Sync for Wormhole {}

impl Default for Wormhole {
    fn default() -> Self {
        Self::new()
    }
}

impl Wormhole {
    pub fn new() -> Self {
        Self(unsafe { wh_create() })
    }
}

impl Drop for Wormhole {
    fn drop(&mut self) {
        unsafe { wh_destroy(self.0) }
    }
}

pub struct WormRef<'a> {
    inner: *mut ffi::c_void,
    _wormhole: PhantomData<&'a Wormhole>,
}

impl WormRef<'_> {
    pub unsafe fn get(&self, key: *const ffi::c_void, key_len: usize) -> Option<u64> {
        unsafe {
            let mut value = 0u64;
            let mut value_len = 0u32;
            wh_get(
                self.inner,
                key,
                key_len as u32,
                &mut value as *mut u64 as _,
                8,
                &mut value_len,
            )
            .then_some(value)
        }
    }

    pub unsafe fn put(&self, key: *const ffi::c_void, key_len: usize, value: u64) {
        unsafe {
            wh_put(
                self.inner,
                key,
                key_len as u32,
                &value as *const u64 as _,
                8,
            );
        }
    }

    pub unsafe fn del(&self, key: *const ffi::c_void, key_len: usize) {
        unsafe { wh_del(self.inner, key, key_len as u32) };
    }

    pub unsafe fn iter(&self, key: *const ffi::c_void, key_len: usize) -> WormIter<'_> {
        unsafe {
            let inner = wh_iter_create(self.inner);
            wh_iter_seek(inner, key, key_len as u32);
            WormIter {
                inner,
                _wormref: PhantomData,
            }
        }
    }
}

impl Drop for WormRef<'_> {
    fn drop(&mut self) {
        unsafe {
            wh_unref(self.inner);
        }
    }
}

pub struct WormIter<'a> {
    inner: *mut ffi::c_void,
    _wormref: PhantomData<&'a ()>,
}

impl Iterator for WormIter<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if !unsafe { wh_iter_valid(self.inner) } {
            return None;
        }

        let mut buffer = [0u8; 8];
        let mut len = 0u32;
        unsafe {
            wh_iter_peek(
                self.inner,
                core::ptr::null_mut(),
                0,
                core::ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
                8,
                &mut len,
            );
            wh_iter_skip1(self.inner);
        }
        Some(u64::from_ne_bytes(buffer))
    }
}

impl Drop for WormIter<'_> {
    fn drop(&mut self) {
        unsafe {
            wh_iter_destroy(self.inner);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::Wormhole;

    #[test]
    fn smoke() {
        let wh = Wormhole::new();
        let wr = wh.pin();

        const COUNT: u64 = 100_000;

        for i in 0..COUNT {
            let key = i.to_be_bytes();
            unsafe { wr.put(key.as_ptr().cast(), 8, i) };
        }

        for i in 0..COUNT {
            let key = i.to_be_bytes();
            assert_eq!(unsafe { wr.get(key.as_ptr().cast(), 8) }, Some(i));
        }

        for i in 0..COUNT {
            let key = i.to_be_bytes();
            unsafe { wr.del(key.as_ptr().cast(), 8) };
        }

        for i in 0..COUNT {
            let key = i.to_be_bytes();
            assert_eq!(unsafe { wr.get(key.as_ptr().cast(), 8) }, None);
        }
    }

    #[test]
    fn iter() {
        let wh = Wormhole::new();
        let wr = wh.pin();

        const COUNT: u64 = 100_000;

        for i in 0..COUNT {
            let key = i.to_be_bytes();
            unsafe { wr.put(key.as_ptr().cast(), 8, i) };
        }

        let start = 50_000u64;
        let start_key = start.to_be_bytes();
        let iter = unsafe { wr.iter(start_key.as_ptr().cast(), 8) };
        for (l, r) in iter.zip(start..COUNT) {
            assert_eq!(l, r);
        }
    }
}
