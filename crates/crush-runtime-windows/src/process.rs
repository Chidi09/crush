use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, ResumeThread,
    STARTUPINFOW, PROCESS_INFORMATION,
    CREATE_SUSPENDED, CREATE_NEW_PROCESS_GROUP,
};
use crate::job_control::JobObject;

pub struct ChildProcess {
    pid: u32,
    process_handle: HANDLE,
}

impl ChildProcess {
    pub fn spawn_in_job(
        command: &str,
        working_dir: Option<&str>,
        job: &JobObject,
    ) -> anyhow::Result<Self> {
        let mut command_wide: Vec<u16> = OsStr::new(command)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let working_dir_wide: Option<Vec<u16>> = working_dir.map(|s| {
            OsStr::new(s)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect()
        });

        let working_dir_ptr = working_dir_wide
            .as_ref()
            .map(|v| v.as_ptr())
            .unwrap_or(null());

        let mut si = unsafe { std::mem::zeroed::<STARTUPINFOW>() };
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

        let mut pi = unsafe { std::mem::zeroed::<PROCESS_INFORMATION>() };

        let flags = CREATE_SUSPENDED | CREATE_NEW_PROCESS_GROUP;

        let success = unsafe {
            CreateProcessW(
                null(),
                command_wide.as_mut_ptr(),
                null(),
                null(),
                0, // Do not inherit handles by default (strict security)
                flags,
                null(),
                working_dir_ptr,
                &si,
                &mut pi,
            )
        };

        if success == 0 {
            return Err(anyhow::anyhow!("Failed to spawn suspended process: {}", std::io::Error::last_os_error()));
        }

        // Immediately assign the process to the Job Object before letting it execute a single instruction
        if let Err(e) = job.assign_process(pi.hProcess) {
            unsafe {
                CloseHandle(pi.hThread);
                CloseHandle(pi.hProcess);
            }
            return Err(e);
        }

        // Resume the main thread, letting the process run locked inside the Job Object
        let resume_result = unsafe { ResumeThread(pi.hThread) };
        if resume_result == u32::MAX {
            let err = std::io::Error::last_os_error();
            unsafe {
                CloseHandle(pi.hThread);
                CloseHandle(pi.hProcess);
            }
            return Err(anyhow::anyhow!("Failed to resume suspended thread: {}", err));
        }

        // We can close the thread handle now since we don't need to control it directly
        unsafe {
            CloseHandle(pi.hThread);
        }

        Ok(Self {
            pid: pi.dwProcessId,
            process_handle: pi.hProcess,
        })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn handle(&self) -> HANDLE {
        self.process_handle
    }
}

impl Drop for ChildProcess {
    fn drop(&mut self) {
        if self.process_handle != 0 && self.process_handle != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.process_handle);
            }
        }
    }
}

unsafe impl Send for ChildProcess {}
unsafe impl Sync for ChildProcess {}
