use std::ffi::CStr;
use std::mem::MaybeUninit;

pub(super) fn local_save_times(timestamps: &[u64]) -> Vec<Option<String>> {
    let locale = TimeLocale::new(c"").or_else(|| TimeLocale::new(c"C"));
    timestamps
        .iter()
        .map(|&seconds| locale.as_ref()?.format(seconds))
        .collect()
}

struct TimeLocale(libc::locale_t);

impl TimeLocale {
    #[allow(unsafe_code)]
    fn new(name: &CStr) -> Option<Self> {
        // A null base creates a private locale, and an empty name reads the environment.
        let locale =
            unsafe { libc::newlocale(libc::LC_TIME_MASK, name.as_ptr(), std::ptr::null_mut()) };
        (!locale.is_null()).then(|| Self(locale))
    }

    #[allow(unsafe_code)]
    fn format(&self, seconds: u64) -> Option<String> {
        let seconds = libc::time_t::try_from(seconds).ok()?;
        let mut local = MaybeUninit::uninit();
        // localtime_r writes to our storage and reports success with a non-null result.
        if unsafe { libc::localtime_r(&seconds, local.as_mut_ptr()) }.is_null() {
            return None;
        }

        let mut buffer = [0_u8; 512];
        // The time was initialized above, the locale remains owned, and the buffer is writable.
        let len = unsafe {
            libc::strftime_l(
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                c"%x %X".as_ptr(),
                local.as_ptr(),
                self.0,
            )
        };
        let label = std::str::from_utf8(buffer.get(..len)?).ok()?.trim();
        (!label.is_empty()).then(|| label.to_owned())
    }
}

impl Drop for TimeLocale {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // newlocale returned this owned handle, and nothing else can free or replace it.
        unsafe { libc::freelocale(self.0) };
    }
}
