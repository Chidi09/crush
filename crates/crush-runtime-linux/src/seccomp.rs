use crush_types::{Result, CrushError};

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
