//! Spawning helpers shared by every child process we start.

use std::time::Duration;

use tokio::process::{Child, Command};

/// How long a child gets to exit after a polite termination request.
pub const GRACE: Duration = Duration::from_secs(2);

/// Windows: no console window for GUI apps. Unix: own process group, so the whole
/// tree (yt-dlp and the ffmpeg it starts) can be stopped together.
pub fn configure(cmd: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        cmd.as_std_mut().process_group(0);
    }
    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = cmd;
    }
}

#[cfg(unix)]
fn signal_group(pid: u32, signal: i32) {
    // SAFETY: plain kill(2) on a process group we created in `configure`; errors (ESRCH) are harmless.
    unsafe {
        libc::kill(-(pid as i32), signal);
    }
}

/// Stop `child` and everything it started: ask nicely, wait `grace`, then force.
pub async fn terminate(child: &mut Child, grace: Duration) {
    #[cfg(unix)]
    {
        let pid = child.id();
        if let Some(pid) = pid {
            signal_group(pid, libc::SIGTERM);
        }
        let _ = tokio::time::timeout(grace, child.wait()).await;
        if let Some(pid) = pid {
            signal_group(pid, libc::SIGKILL); // stragglers such as a lingering ffmpeg
        }
        let _ = child.wait().await;
    }
    #[cfg(windows)]
    {
        let _ = grace;
        if let Some(pid) = child.id() {
            let mut kill = Command::new("taskkill");
            kill.args(["/PID", &pid.to_string(), "/T", "/F"]);
            kill.creation_flags(0x0800_0000);
            let _ = kill.status().await;
        }
        let _ = child.wait().await;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = grace;
        let _ = child.kill().await;
    }
}
