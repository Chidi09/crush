# Smart Stacktrace Intercept & AI Diagnosis

One of the most powerful features of Crush is its built-in, intelligent error diagnosis and recovery engine. It is designed to turn cryptic compiler and runtime failures into actionable, automated fixes.

---

## Smart Stacktrace Intercept

When a containerized process crashes or a compilation stage fails:
1. **Stdout/Stderr Intercept**: Crush actively intercepts the standard error stream of the failing container sandbox.
2. **Crash Fingerprinting**: It parses the crash dump or compiler error (such as Rust `panic!`, Node.js unhandled exceptions, Python stacktraces, or segmentation faults) using built-in log parsers.
3. **Context Gathering**: The runtime captures relevant surrounding context:
   - The detected framework and stack configurations.
   - Recent code modifications or dependency changes.
   - System resource usage metrics (to detect OOM errors).

---

## AI-Powered Claude Diagnostics

Once the crash signature is fingerprint-extracted:
- **Local Diagnostics Engine**: Crush passes the diagnostic signature and file context to the integrated AI engine (powered by Anthropic Claude).
- **Insightful Explanations**: The engine analyzes the trace and explains *exactly* why the failure occurred in simple, human-readable terms.
- **Automated Fix Generation**: Rather than just explaining the bug, the system generates precise code diffs or command recommendations to resolve the issue.
- **Interactive Patching**: Using the Crush GUI or CLI watch mode, developers can review and apply recommended bug fixes with a single click or keyboard command.
