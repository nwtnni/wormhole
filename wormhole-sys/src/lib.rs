use core::ffi;
use core::marker::PhantomData;

unsafe extern "C" {
    fn wh_create() -> *const ffi::c_void;
    fn wh_destroy(wormhole: *const ffi::c_void);

    unsafe fn wh_ref(wormhole: *const ffi::c_void) -> *const ffi::c_void;
    unsafe fn wh_unref(wormref: *const ffi::c_void);

    unsafe fn wh_put(
        wormref: *const ffi::c_void,
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

    unsafe fn wh_del(wormref: *const ffi::c_void, kbuf: *const ffi::c_void, klen: u32) -> bool;
}

pub struct Wormhole(*const ffi::c_void);

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
    inner: *const ffi::c_void,
    _wormhole: PhantomData<&'a Wormhole>,
}

impl WormRef<'_> {
    pub unsafe fn get(&self, key: *const ffi::c_void, key_len: usize) -> Option<u64> {
        unsafe {
            let mut value = 0u64;
            let mut value_len = 0u64;
            wh_get(
                self.inner,
                key,
                key_len as u32,
                &mut value as *mut u64 as _,
                8,
                &mut value_len as *mut u64 as _,
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
}

impl Drop for WormRef<'_> {
    fn drop(&mut self) {
        unsafe {
            wh_unref(self.inner);
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
            unsafe { wr.put(&i as *const _ as _, 8, i) };
        }

        for i in 0..COUNT {
            assert_eq!(unsafe { wr.get(&i as *const _ as _, 8) }, Some(i));
        }

        for i in 0..COUNT {
            unsafe { wr.del(&i as *const _ as _, 8) };
        }

        for i in 0..COUNT {
            assert_eq!(unsafe { wr.get(&i as *const _ as _, 8) }, None);
        }
    }
}
