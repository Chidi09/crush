use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::JobObjects::{
    CreateJobObjectW, SetInformationJobObject, AssignProcessToJobObject,
    JobObjectExtendedLimitInformation, JobObjectCpuRateControlInformation,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
    JOB_OBJECT_CPU_RATE_CONTROL_ENABLE, JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
};

pub struct JobObject {
    handle: HANDLE,
}

impl JobObject {
    pub fn create(name: &str) -> anyhow::Result<Self> {
        let wide_name: Vec<u16> = OsStr::new(name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let handle = unsafe {
            CreateJobObjectW(null(), wide_name.as_ptr())
        };

        if handle == 0 {
            return Err(anyhow::anyhow!("Failed to create Job Object: {}", std::io::Error::last_os_error()));
        }

        let job = Self { handle };

        // By default, make sure the job terminates all processes inside it when the job handle closes
        job.enable_kill_on_close()?;

        Ok(job)
    }

    fn enable_kill_on_close(&self) -> anyhow::Result<()> {
        let mut info = unsafe { std::mem::zeroed::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        let result = unsafe {
            SetInformationJobObject(
                self.handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };

        if result == 0 {
            return Err(anyhow::anyhow!("Failed to set JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: {}", std::io::Error::last_os_error()));
        }

        Ok(())
    }

    pub fn set_memory_limit(&self, max_bytes: u64) -> anyhow::Result<()> {
        let mut info = unsafe { std::mem::zeroed::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_JOB_MEMORY | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
        info.JobMemoryLimit = max_bytes as usize;
        info.ProcessMemoryLimit = max_bytes as usize;

        let result = unsafe {
            SetInformationJobObject(
                self.handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };

        if result == 0 {
            return Err(anyhow::anyhow!("Failed to set memory limit: {}", std::io::Error::last_os_error()));
        }

        Ok(())
    }

    pub fn set_cpu_limit(&self, percentage: u32) -> anyhow::Result<()> {
        // Percentage can be between 1 and 100
        let rate_control = (percentage * 100) as u32; // Windows expects rate in 1/100 of 1% (e.g. 10000 = 100%)
        
        let mut info = unsafe { std::mem::zeroed::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() };
        info.ControlFlags = JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP;
        info.Anonymous.CpuRate = rate_control;

        let result = unsafe {
            SetInformationJobObject(
                self.handle,
                JobObjectCpuRateControlInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
            )
        };

        if result == 0 {
            return Err(anyhow::anyhow!("Failed to set CPU rate limits: {}", std::io::Error::last_os_error()));
        }

        Ok(())
    }

    pub fn assign_process(&self, process_handle: HANDLE) -> anyhow::Result<()> {
        let result = unsafe {
            AssignProcessToJobObject(self.handle, process_handle)
        };

        if result == 0 {
            return Err(anyhow::anyhow!("Failed to assign process to Job Object: {}", std::io::Error::last_os_error()));
        }

        Ok(())
    }

    pub fn handle(&self) -> HANDLE {
        self.handle
    }
}

impl Drop for JobObject {
    fn drop(&mut self) {
        if self.handle != 0 && self.handle != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

unsafe impl Send for JobObject {}
unsafe impl Sync for JobObject {}
