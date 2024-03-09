// inspired by @fasterthanlime's brilliant post https://fasterthanli.me/articles/a-terminal-case-of-linux
// and Jakub Kądziołka's great follow up https://compilercrim.es/amos-nerdsniped-me/

use anyhow::{bail, Result};
use std::convert::TryFrom;
use tokio::{io::AsyncReadExt, process::Command};
use tokio_fd::AsyncFd;

pub async fn run(cmd: &mut Command) -> Result<()> {
    let (primary_fd, secondary_fd) = open_terminal();

    unsafe {
        cmd.pre_exec(move || {
            if libc::login_tty(secondary_fd) != 0 {
                panic!("couldn't set the controlling terminal or something");
            }
            Ok(())
        })
    };
    let mut child = cmd.spawn()?;

    let mut buf = vec![0u8; 1024];
    let mut primary = AsyncFd::try_from(primary_fd)?;

    loop {
        tokio::select! {
            n = primary.read(&mut buf) => {
                let n = n?;
                let slice = &buf[..n];

                let s = std::str::from_utf8(slice)?;
                print!("{}", s);
            },

            status = child.wait() => {
                match status {
                    Ok(s) => {
                        if s.success() {
                            break;
                        }
                        bail!("command failed with {}", s)
                    }
                    Err(e) => bail!(e),
                }
            },
        }
    }

    Ok(())
}

fn open_terminal() -> (i32, i32) {
    let mut primary_fd: i32 = -1;
    let mut secondary_fd: i32 = -1;
    unsafe {
        let ret = libc::openpty(
            &mut primary_fd,
            &mut secondary_fd,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if ret != 0 {
            panic!("Failed to openpty!");
        }
    };
    (primary_fd, secondary_fd)
}
