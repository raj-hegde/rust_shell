use std::{
    ffi::{CString, c_char, c_int},
    io::{self},
    os::unix::raw::pid_t,
    path::PathBuf,
    ptr::null,
};

use libc::{
    _exit, O_APPEND, O_CREAT, O_RDONLY, O_WRONLY, close, dup2, execvp, fork, open, waitpid,
};

use crate::parser::{Command, Pipeline, Redirection};

pub fn execute_pipeline(pipeline: Pipeline) -> Result<(), String> {
    let num_commands = pipeline.commands.len();
    if num_commands == 0 {
        return Ok(());
    }

    if num_commands == 1 && try_run_builtin(&pipeline.commands[0])? {
        return Ok(());
    }

    let mut pipes: Vec<[c_int; 2]> = Vec::new();
    for _ in 0..(num_commands - 1) {
        let mut fds = [0 as c_int; 2];
        let ret = unsafe { libc::pipe(fds.as_mut_ptr()) };
        if ret < 0 {
            close_all_pipes(&pipes);
            return Err(format!(
                "pipe creation failed: {}",
                io::Error::last_os_error()
            ));
        }
        pipes.push(fds);
    }

    let mut child_pids: Vec<pid_t> = Vec::with_capacity(num_commands);

    for (i, cmd) in pipeline.commands.iter().enumerate() {
        let pid = unsafe { fork() };

        match pid {
            0 => {
                if i > 0 {
                    let prev_read_fd = pipes[i - 1][0];
                    if unsafe { dup2(prev_read_fd, 0) } < 0 {
                        unsafe { _exit(1) };
                    }
                }

                if i < num_commands - 1 {
                    let next_write_fd = pipes[i][1];
                    if unsafe { dup2(next_write_fd, 1) } < 0 {
                        unsafe { _exit(1) };
                    }
                }

                close_all_pipes(&pipes);

                if let Err(e) = apply_redirections(&cmd.redirects) {
                    eprintln!("Redirection error: {e}");
                    unsafe { _exit(1) };
                }

                exec_external_command(cmd);
            }

            p if p > 0 => {
                child_pids.push(p);
            }

            _ => {
                close_all_pipes(&pipes);
                return Err(format!("fork failed: {}", io::Error::last_os_error()));
            }
        }
    }

    close_all_pipes(&pipes);

    if pipeline.background {
        if let Some(last_pid) = child_pids.last() {
            println!("[Background PID: {}]", last_pid);
        }
    } else {
        for pid in child_pids {
            let mut status = 0;
            unsafe { waitpid(pid, &mut status, 0) };
        }
    }
    Ok(())
}

fn close_all_pipes(pipes: &[[c_int; 2]]) {
    for pipe in pipes {
        unsafe {
            close(pipe[0]);
            close(pipe[1]);
        }
    }
}

fn apply_redirections(redirects: &[Redirection]) -> Result<(), String> {
    for redir in redirects {
        match redir {
            Redirection::Read(path) => {
                let c_path = CString::new(path.as_str()).map_err(|e| e.to_string())?;
                let fd = unsafe { open(c_path.as_ptr(), O_RDONLY) };
                if fd < 0 {
                    return Err(format!(
                        "cannot open '{}' for reading: {}",
                        path,
                        io::Error::last_os_error()
                    ));
                }
                if unsafe { dup2(fd, 1) } < 0 {
                    return Err(format!("failed to redirect stdout to '{}'", path));
                }
                unsafe { close(fd) };
            }
            Redirection::Write(path) => {
                let c_path = CString::new(path.as_str()).map_err(|e| e.to_string())?;
                let fd = unsafe { open(c_path.as_ptr(), O_WRONLY | O_CREAT | O_APPEND, 0o644) };
                if fd < 0 {
                    return Err(format!(
                        "cannot open '{}' for writing: {}",
                        path,
                        io::Error::last_os_error()
                    ));
                }
                if unsafe { dup2(fd, 1) } < 0 {
                    return Err(format!("failed to redirect stdout to '{}'", path));
                }
                unsafe { close(fd) };
            }
            Redirection::Append(path) => {
                let c_path = CString::new(path.as_str()).map_err(|e| e.to_string())?;
                let fd = unsafe { open(c_path.as_ptr(), O_WRONLY | O_CREAT | O_APPEND, 0o644) };
                if fd < 0 {
                    return Err(format!(
                        "cannot open '{}' for reading: {}",
                        path,
                        io::Error::last_os_error()
                    ));
                }
                if unsafe { dup2(fd, 1) } < 0 {
                    return Err(format!("failed to redirect stdout to '{}'", path));
                }
                unsafe { close(fd) };
            }
        }
    }
    Ok(())
}

fn exec_external_command(cmd: &Command) -> ! {
    let c_args: Vec<CString> = cmd
        .args
        .iter()
        .map(|arg| CString::new(arg.as_str()).unwrap_or_default())
        .collect();
    let mut argv: Vec<*const c_char> = c_args.iter().map(|c| c.as_ptr()).collect();
    argv.push(null());

    unsafe {
        execvp(argv[0], argv.as_ptr());
        eprintln!("{}: {}", cmd.args[0], io::Error::last_os_error());
        _exit(127);
    }
}

fn try_run_builtin(cmd: &Command) -> Result<bool, String> {
    match cmd.args[0].as_str() {
        "cd" => {
            let target = match cmd.args.get(1) {
                Some(dir) => PathBuf::from(dir),
                None => match std::env::var("HOME") {
                    Ok(ref home) => PathBuf::from(home),
                    Err(_) => PathBuf::from("/"),
                },
            };
            if let Err(e) = std::env::set_current_dir(&target) {
                eprintln!("cd: {}: {e}", target.display());
            }
            Ok(true)
        }
        "exit" => {
            let code = cmd
                .args
                .get(1)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            std::process::exit(code);
        }
        _ => Ok(false),
    }
}
