// Windows Job Object wrapper. Lets us guarantee that every child process
// crush spawns dies when crush itself exits — Ctrl+C, panic, OOM, or just
// closing the terminal. Without this, deep monorepo process trees
// (cmd.exe → pnpm → node → turbo → node workers) routinely orphan and
// keep listening on ports.
//
// On non-Windows, this is a no-op shell so call sites stay tidy.

#[cfg(windows)]
mod imp {
    use std::sync::OnceLock;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
        JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    pub struct Job(HANDLE);
    // HANDLE is isize in windows-sys; trivially Send + Sync but the impls
    // make the intent explicit.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    impl Drop for Job {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0); }
        }
    }

    static JOB: OnceLock<Option<Job>> = OnceLock::new();

    fn create() -> Option<Job> {
        unsafe {
            let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if h == 0 { return None; }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                h,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                CloseHandle(h);
                return None;
            }
            Some(Job(h))
        }
    }

    pub fn init() {
        JOB.get_or_init(create);
    }

    fn assign_raw(raw: HANDLE) {
        let job = match JOB.get().and_then(|j| j.as_ref()) {
            Some(j) => j,
            None => return,
        };
        unsafe {
            // If assignment fails (e.g. parent already in an unbreakable job),
            // silently fall back to no-op rather than killing the flow.
            let _ = AssignProcessToJobObject(job.0, raw);
        }
    }

    pub fn assign(child: &tokio::process::Child) {
        if let Some(raw) = child.raw_handle() {
            assign_raw(raw as HANDLE);
        }
    }

    pub fn assign_std(child: &std::process::Child) {
        use std::os::windows::io::AsRawHandle;
        assign_raw(child.as_raw_handle() as HANDLE);
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn init() {}
    pub fn assign(_child: &tokio::process::Child) {}
    pub fn assign_std(_child: &std::process::Child) {}
}

pub use imp::{assign, assign_std, init};
