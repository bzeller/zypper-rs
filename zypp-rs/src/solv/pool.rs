use libsolv_sys::ffi as raw;
use std::ffi::{CStr,CString, NulError};

use super::repository::Repository;

pub type Id = raw::Id;

#[derive(Debug)]
pub struct Pool {
    pool: *mut raw::Pool
}


impl Pool {

    pub( crate ) fn new_from_ptr ( pool: *mut raw::Pool ) -> Self {
        Pool { pool }
    }

    pub fn new() -> Self {
        unsafe {
            let pool = raw::pool_create();
            if pool.is_null() {
                panic!("Failed to init pool");
            }
            Pool { pool }
        }
    }

    pub fn set_rootdir( &mut self, rootdir: &str ) -> Result<(), NulError> {
        unsafe {
            if rootdir.is_empty() {
                raw::pool_set_rootdir( self.pool, std::ptr::null() );
            } else {
                let c_str = match CString::new( rootdir ) {
                    Ok(s) => s,
                    Err(e) => return Err(e)
                };
                raw::pool_set_rootdir( self.pool, c_str.as_ptr() );
            }
        }
        Ok(())
    }

    pub fn get_rootdir ( &mut self ) -> String {
        unsafe {
            // careful rootdir can be NULL
            let unsafe_str =  raw::pool_get_rootdir(self.pool);
            if unsafe_str.is_null() {
                return String::new();
            }
            let cstr = CStr::from_ptr( unsafe_str );
            String::from_utf8_lossy(cstr.to_bytes()).to_string()
        }
    }

    pub fn delete_repository ( &mut self, repo: Repository ) {
        unsafe {
            if self.pool != (*repo.repo).pool {
                return;
            }
            raw::repo_free( repo.repo, 0 );
            if (*self.pool).nrepos == 0 {
                raw::pool_freeallrepos( self.pool, 1 );
            }
        }
    }

    pub fn add_solvable ( &mut self ) -> Id {
        unsafe {
            return raw::pool_add_solvable( self.pool );
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        unsafe { raw::pool_free( self.pool ); }
    }
}
