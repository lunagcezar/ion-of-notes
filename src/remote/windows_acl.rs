//! Windows file ACL adapter for the remembered remote session.
//!
//! Unix has `chmod 0600`; on Windows the equivalent is a DACL that grants the
//! current user full control and removes inherited entries. The call is
//! best-effort at the API level: a failure is returned to the caller, which
//! aborts the session write rather than storing a token in a permissive file.

use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, GENERIC_ALL, HANDLE};
use windows_sys::Win32::Security::Authorization::{SE_FILE_OBJECT, SetNamedSecurityInfoW};
use windows_sys::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACL, ACL_REVISION, AddAccessAllowedAce, DACL_SECURITY_INFORMATION,
    GetLengthSid, GetTokenInformation, InitializeAcl, InitializeSecurityDescriptor,
    SECURITY_DESCRIPTOR, SetSecurityDescriptorDacl, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// `SECURITY_DESCRIPTOR_REVISION`; kept local to avoid a broad feature flag.
const DESCRIPTOR_REVISION: u32 = 1;

/// Restrict `path` to the current user (full control) and remove inherited
/// entries.
pub(super) fn restrict_to_current_user(path: &Path) -> io::Result<()> {
    let mut token: HANDLE = std::ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }

    let result = (|| -> io::Result<()> {
        let mut size = 0u32;
        unsafe {
            GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut size);
        }
        if size == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut user_buffer = vec![0u8; size as usize];
        if unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                user_buffer.as_mut_ptr().cast(),
                size,
                &mut size,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let token_user = user_buffer.as_ptr() as *const TOKEN_USER;
        let sid = unsafe { (*token_user).User.Sid };
        let sid_length = unsafe { GetLengthSid(sid) } as usize;

        let acl_size = std::mem::size_of::<ACL>() + std::mem::size_of::<ACCESS_ALLOWED_ACE>()
            - std::mem::size_of::<u32>()
            + sid_length;
        let mut acl_buffer = vec![0u8; acl_size];
        let acl = acl_buffer.as_mut_ptr() as *mut ACL;
        if unsafe { InitializeAcl(acl, acl_size as u32, ACL_REVISION) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { AddAccessAllowedAce(acl, ACL_REVISION, GENERIC_ALL, sid) } == 0 {
            return Err(io::Error::last_os_error());
        }

        let mut descriptor: SECURITY_DESCRIPTOR = unsafe { std::mem::zeroed() };
        let descriptor_ptr = &mut descriptor as *mut SECURITY_DESCRIPTOR as *mut std::ffi::c_void;
        if unsafe { InitializeSecurityDescriptor(descriptor_ptr, DESCRIPTOR_REVISION) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { SetSecurityDescriptorDacl(descriptor_ptr, 1, acl, 0) } == 0 {
            return Err(io::Error::last_os_error());
        }

        let wide: Vec<u16> = OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let status = unsafe {
            SetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                acl,
                std::ptr::null(),
            )
        };
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        Ok(())
    })();

    unsafe {
        CloseHandle(token);
    }
    result
}
