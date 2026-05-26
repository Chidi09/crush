use crush_types::{Result, CrushError};

fn syscall_name_to_number(name: &str) -> Option<u32> {
    #[cfg(target_arch = "x86_64")]
    let table: &[(&str, u32)] = &[
        ("read", 0), ("write", 1), ("open", 2), ("close", 3), ("stat", 4), ("fstat", 5),
        ("lstat", 6), ("poll", 7), ("lseek", 8), ("mmap", 9), ("mprotect", 10), ("munmap", 11),
        ("brk", 12), ("rt_sigaction", 13), ("rt_sigprocmask", 14), ("ioctl", 16), ("pread64", 17),
        ("pwrite64", 18), ("readv", 19), ("writev", 20), ("access", 21), ("pipe", 22),
        ("select", 23), ("sched_yield", 24), ("mremap", 25), ("msync", 26), ("mincore", 27),
        ("madvise", 28), ("dup", 32), ("dup2", 33), ("nanosleep", 35), ("getpid", 39),
        ("socket", 41), ("connect", 42), ("accept", 43), ("sendto", 44), ("recvfrom", 45),
        ("sendmsg", 46), ("recvmsg", 47), ("bind", 49), ("listen", 50), ("getsockname", 51),
        ("getpeername", 52), ("socketpair", 53), ("setsockopt", 54), ("getsockopt", 55),
        ("clone", 56), ("fork", 57), ("vfork", 58), ("execve", 59), ("exit", 60),
        ("wait4", 61), ("kill", 62), ("uname", 63), ("fcntl", 72), ("flock", 73),
        ("fsync", 74), ("fdatasync", 75), ("truncate", 76), ("ftruncate", 77),
        ("getdents", 78), ("getcwd", 79), ("chdir", 80), ("fchdir", 81), ("rename", 82),
        ("mkdir", 83), ("rmdir", 84), ("creat", 85), ("link", 86), ("unlink", 87),
        ("symlink", 88), ("readlink", 89), ("chmod", 90), ("fchmod", 91), ("chown", 92),
        ("fchown", 93), ("lchown", 94), ("umask", 95), ("gettimeofday", 96), ("getrlimit", 97),
        ("getrusage", 98), ("sysinfo", 99), ("times", 100), ("getuid", 102), ("getgid", 104),
        ("geteuid", 107), ("getegid", 108), ("setpgid", 109), ("getppid", 110), ("getpgrp", 111),
        ("setsid", 112), ("setreuid", 113), ("setregid", 114), ("getgroups", 115),
        ("setgroups", 116), ("setresuid", 117), ("getresuid", 118), ("setresgid", 119),
        ("getresgid", 120), ("getpgid", 121), ("setfsuid", 122), ("setfsgid", 123),
        ("getsid", 124), ("capget", 125), ("capset", 126), ("rt_sigpending", 127),
        ("rt_sigtimedwait", 128), ("rt_sigqueueinfo", 129), ("rt_sigsuspend", 130),
        ("sigaltstack", 131), ("utime", 132), ("mknod", 133), ("statfs", 137), ("fstatfs", 138),
        ("prctl", 157), ("arch_prctl", 158), ("gettid", 186), ("futex", 202), ("set_tid_address", 218),
        ("clock_gettime", 228), ("clock_nanosleep", 230), ("exit_group", 231),
        ("epoll_create", 213), ("epoll_ctl", 233), ("epoll_wait", 232), ("epoll_pwait", 281),
        ("openat", 257), ("mkdirat", 258), ("fstatat", 262), ("unlinkat", 263),
        ("renameat", 264), ("linkat", 265), ("symlinkat", 266), ("readlinkat", 267),
        ("fchmodat", 268), ("faccessat", 269), ("pselect6", 270), ("ppoll", 271),
        ("set_robust_list", 273), ("get_robust_list", 274), ("splice", 275), ("tee", 276),
        ("sync_file_range", 277), ("vmsplice", 278), ("move_pages", 279),
        ("accept4", 288), ("pipe2", 293), ("dup3", 292), ("inotify_init1", 294),
        ("preadv", 295), ("pwritev", 296), ("recvmmsg", 299), ("prlimit64", 302),
        ("sendmmsg", 307), ("getrandom", 318), ("memfd_create", 319),
        ("copy_file_range", 326), ("preadv2", 327), ("pwritev2", 328),
        ("close_range", 436), ("openat2", 437), ("faccessat2", 439),
    ];
    #[cfg(target_arch = "aarch64")]
    let table: &[(&str, u32)] = &[
        ("read", 63), ("write", 64), ("open", 1024), ("close", 57), ("stat", 1038),
        ("fstat", 80), ("lstat", 1039), ("poll", 73), ("lseek", 62), ("mmap", 222),
        ("mprotect", 226), ("munmap", 215), ("brk", 214), ("ioctl", 29), ("readv", 65),
        ("writev", 66), ("access", 1033), ("pipe", 1040), ("dup", 23), ("dup3", 24),
        ("nanosleep", 101), ("getpid", 172), ("socket", 198), ("connect", 203),
        ("accept", 202), ("sendto", 206), ("recvfrom", 207), ("sendmsg", 211),
        ("recvmsg", 212), ("bind", 200), ("listen", 201), ("getsockname", 204),
        ("getpeername", 205), ("socketpair", 199), ("setsockopt", 208), ("getsockopt", 209),
        ("clone", 220), ("execve", 221), ("exit", 93), ("wait4", 260), ("kill", 129),
        ("uname", 160), ("fcntl", 25), ("fsync", 82), ("fdatasync", 83), ("truncate", 45),
        ("ftruncate", 46), ("getcwd", 17), ("chdir", 49), ("fchdir", 50), ("rename", 1034),
        ("mkdir", 1030), ("rmdir", 1035), ("unlink", 1026), ("symlink", 1036),
        ("readlink", 1037), ("chmod", 1028), ("fchmod", 52), ("chown", 1029),
        ("fchown", 55), ("umask", 166), ("gettimeofday", 169), ("getuid", 174),
        ("getgid", 176), ("geteuid", 175), ("getegid", 177), ("getppid", 173),
        ("setsid", 157), ("setreuid", 145), ("setregid", 143), ("getgroups", 158),
        ("setgroups", 159), ("getpgid", 155), ("gettid", 178), ("futex", 98),
        ("set_tid_address", 96), ("clock_gettime", 113), ("clock_nanosleep", 115),
        ("exit_group", 94), ("epoll_create", 1042), ("epoll_ctl", 21), ("epoll_pwait", 22),
        ("openat", 56), ("mkdirat", 34), ("fstatat", 79), ("unlinkat", 35),
        ("renameat", 38), ("linkat", 37), ("symlinkat", 36), ("readlinkat", 78),
        ("fchmodat", 53), ("faccessat", 48), ("pipe2", 59), ("inotify_init1", 26),
        ("preadv", 69), ("pwritev", 70), ("recvmmsg", 243), ("sendmmsg", 269),
        ("getrandom", 278), ("memfd_create", 279), ("copy_file_range", 285),
        ("close_range", 436), ("openat2", 437), ("faccessat2", 439),
    ];
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let table: &[(&str, u32)] = &[];
    table.iter().find(|(n, _)| *n == name).map(|(_, nr)| *nr)
}

pub const AUDIT_ARCH_X86_64: u32 = 0xc000003e;
pub const AUDIT_ARCH_AARCH64: u32 = 0xc00000b7;

const SECCOMP_RET_KILL_PROCESS: u32 = 0x80000000;
const SECCOMP_RET_ERRNO: u32 = 0x00050000;
const SECCOMP_RET_ALLOW: u32 = 0x7fff0000;

#[repr(C)]
struct SockFilter { code: u16, jt: u8, jf: u8, k: u32 }

const BPF_LD_W_ABS: u16 = 0x20;
const BPF_JMP_JEQ: u16 = 0x15;
const BPF_RET: u16 = 0x06;

pub struct SeccompFilterCompiler { arch: u32 }

impl SeccompFilterCompiler {
    pub fn new() -> Self {
        #[cfg(target_arch = "x86_64")]
        let arch = AUDIT_ARCH_X86_64;
        #[cfg(target_arch = "aarch64")]
        let arch = AUDIT_ARCH_AARCH64;
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        let arch = AUDIT_ARCH_X86_64;
        Self { arch }
    }

    pub fn compile_bpf_filter(&self, allow_list: &[String], blocked_syscalls_count: &mut usize) -> Result<Vec<u8>> {
        let allowed: Vec<u32> = allow_list.iter()
            .filter_map(|name| syscall_name_to_number(name))
            .collect();

        // BPF program layout:
        // [0] LD arch from seccomp_data[4]
        // [1] JEQ expected_arch → jump to [3], else fall through to [2]
        // [2] RET KILL_PROCESS  (arch mismatch = kill)
        // [3] LD syscall_nr from seccomp_data[0]
        // [4..N] linear allowlist: JEQ sysno → ALLOW, else continue
        // [N+1] RET KILL_PROCESS (default: kill)
        let mut bpf = Vec::new();
        bpf.push(SockFilter { code: BPF_LD_W_ABS, jt: 0, jf: 0, k: 4 });          // [0] load arch
        bpf.push(SockFilter { code: BPF_JMP_JEQ, jt: 1, jf: 0, k: self.arch });    // [1] arch match → skip [2] to reach [3], mismatch → fall through to [2]
        bpf.push(SockFilter { code: BPF_RET, jt: 0, jf: 0, k: SECCOMP_RET_KILL_PROCESS }); // [2] arch mismatch = kill
        bpf.push(SockFilter { code: BPF_LD_W_ABS, jt: 0, jf: 0, k: 0 });          // [3] load syscall nr

        // Classic BPF allowlist: jt=0 falls through to RET ALLOW (match), jf=1 skips it (no match).
        // Both jt and jf are offsets from the instruction *after* the jump — so jt=0 means the
        // very next instruction (RET ALLOW), and jf=1 means skip that one and reach the next JEQ.
        for &sysno in &allowed {
            bpf.push(SockFilter { code: BPF_JMP_JEQ, jt: 0, jf: 1, k: sysno });
            bpf.push(SockFilter { code: BPF_RET, jt: 0, jf: 0, k: SECCOMP_RET_ALLOW });
        }

        // Default: kill the process for any non-allowlisted syscall
        bpf.push(SockFilter { code: BPF_RET, jt: 0, jf: 0, k: SECCOMP_RET_KILL_PROCESS });

        *blocked_syscalls_count = allowed.len();

        let bytecode: Vec<u8> = unsafe {
            std::slice::from_raw_parts(
                bpf.as_ptr() as *const u8,
                bpf.len() * std::mem::size_of::<SockFilter>(),
            ).to_vec()
        };
        Ok(bytecode)
    }

    /// Apply seccomp BPF to the calling process.
    /// ONLY call this inside a `pre_exec` hook (child process, after fork, before execve).
    /// Calling this in the daemon parent restricts the daemon's own syscalls, not containers'.
    #[cfg(target_os = "linux")]
    pub fn apply_filter(&self, bytecode: &[u8]) -> Result<()> {
        if bytecode.len() < std::mem::size_of::<SockFilter>() {
            return Err(CrushError::SeccompError("Empty BPF bytecode".to_string()));
        }
        let prog = libc::sock_fprog {
            len: (bytecode.len() / std::mem::size_of::<SockFilter>()) as u16,
            filter: bytecode.as_ptr() as *mut libc::sock_filter,
        };
        // SECCOMP_MODE_FILTER = 2 (BPF program). Mode 1 is SECCOMP_MODE_STRICT (no program).
        let result = unsafe {
            libc::prctl(libc::PR_SET_SECCOMP, 2u64, &prog as *const _ as u64)
        };
        if result != 0 {
            return Err(CrushError::SeccompError(format!(
                "PR_SET_SECCOMP(FILTER) failed: {}", std::io::Error::last_os_error()
            )));
        }
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn apply_filter(&self, _bytecode: &[u8]) -> Result<()> { Ok(()) }
}
