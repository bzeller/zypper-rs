use solv_sys as raw;
//use std::ffi::{CStr,CString, NulError};
use super::pool::Pool;

#[derive(Debug)]
pub struct Repository {
    pub( crate ) repo: *mut raw::Repo
}

impl Repository {

    pub( crate ) fn new_from_ptr ( repo: *mut raw::Repo ) -> Self {
        return Repository { repo }
    }

    pub fn get_pool ( &self ) -> Pool {
        unsafe {
            return Pool::new_from_ptr( (*self.repo).pool );
        }
    }
}
