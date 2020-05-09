use alloc::boxed::Box;
use alloc::collections::LinkedList;
use alloc::string::{String, ToString};
use core::cell::RefCell;

use usrlib::syscalls::{sys_close, sys_spawn_domain, sys_open};
use usr_interfaces::xv6::Thread;
use usr_interfaces::vfs::FileMode;

#[derive(Debug)]
pub struct Redir {
    stdin: usize,
    stdout: usize,
    stderr: usize,
}

impl Redir {
    pub fn new() -> Self {
        Self {
            stdin: 0,
            stdout: 1,
            stderr: 2,
        }
    }

    fn copy(&self) -> Self {
        Self {
            stdin: self.stdin,
            stdout: self.stdout,
            stderr: self.stderr,
        }
    }
}


pub trait Command: core::fmt::Debug {
    fn run(&self, redir: Redir) -> LinkedList<Box<dyn Thread>>;
}

impl dyn Command {
    pub fn parse(line: &str) -> (Box<dyn Command>, &str) {
        PipeCommand::parse(line)
    }
}

#[derive(Debug)]
struct NoopCommand {}

impl Command for NoopCommand {
    fn run(&self, _redir: Redir) -> LinkedList<Box<dyn Thread>> {
        LinkedList::new()
    }
}

#[derive(Debug)]
struct ExecCommand {
    cmd: String,
    args: String,
}

impl ExecCommand {
    fn new(line: &str) -> Self {
        Self {
            cmd: line.split_ascii_whitespace().next().unwrap().to_string(),
            args: line.to_string(),
        }
    }

    fn parse(line: &str) -> (Box<dyn Command>, &str) {
        let line = line.strip_suffix(char::is_whitespace).unwrap_or(line);

        if line.is_empty() {
            return (box NoopCommand{}, line);
        }

        assert!(line.find(&['|', '<', '>'][..]).is_none());
        (box Self::new(line), &line[line.len()..])
    }
}

impl Command for ExecCommand {
    fn run(&self, redir: Redir) -> LinkedList<Box<dyn Thread>> {
        let result = sys_spawn_domain(&self.cmd, &self.args, &[Some(redir.stdin), Some(redir.stdout), Some(redir.stderr)]);
        let mut ll = LinkedList::new();
        ll.push_back(result.unwrap());
        ll
    }
}

#[derive(Debug)]
pub struct PipeCommand {
    left: Box<dyn Command>,
    right: Box<dyn Command>,
}

impl PipeCommand {
    fn new(left: Box<dyn Command>, right: Box<dyn Command>) -> Self {
        Self {
            left,
            right,
        }
    }

    fn parse(line: &str) -> (Box<dyn Command>, &str) {
        let (left_str, right_str) = line.split_at(line.find('|').unwrap_or(line.len()));

        let (left_cmd, left_leftover) = RedirCommand::parse(left_str);
        assert!(left_leftover.is_empty());
        
        match right_str.is_empty() {
            false => {
                let (right_cmd, right_leftover) = Self::parse(&right_str[1..]);
                (box Self::new(left_cmd, right_cmd), right_leftover)
            },
            true => (left_cmd, left_leftover)
        }
    }
}

impl Command for PipeCommand {
    fn run(&self, redir: Redir) -> LinkedList<Box<dyn Thread>> {
        // Setup redirection
        let (rfd, wfd) = usrlib::syscalls::sys_pipe().unwrap();
        let mut left_redir = redir.copy();
        left_redir.stdout = wfd;
        let mut right_redir = redir.copy();
        right_redir.stdin = rfd;

        // Run commands
        let mut result = self.left.run(left_redir);
        result.append(&mut self.right.run(right_redir));

        // Cleanup
        // We are safe to close these fds here because the fdtable is already saved
        // when sys_spawn_domain returns
        sys_close(rfd).unwrap();
        sys_close(wfd).unwrap();
        result
    }
}

#[derive(Debug)]
pub struct RedirCommand {
    cmd: Box<dyn Command>,
    file: String,
    mode: FileMode,
    fd: usize,
}

impl RedirCommand {
    fn new(cmd: Box<dyn Command>, file: &str, mode: FileMode, fd: usize) -> Self {
        Self {
            cmd,
            file: file.to_string(),
            mode,
            fd,
        }
    }

    // We only allow the redir part comes after everything else in the command
    // For example, `ls -a > foo` is allowed but `ls > foo -a' is not allowed
    fn parse(line: &str) -> (Box<dyn Command>, &str) {
        // Split the redir part from the reset of the string
        let (exec_str, mut redir_str) = line.split_at(line.find(&['<', '>'][..]).unwrap_or(line.len()));

        // Parse the possible exec cmd
        let (mut subcmd, exec_left_over) = ExecCommand::parse(exec_str);
        assert!(exec_left_over.is_empty());

        // Process the redir_str
        while !redir_str.is_empty() {
            // Strip leading whitespaces
            redir_str = redir_str.strip_prefix(char::is_whitespace).unwrap_or(redir_str);

            // Get redir token
            let (redir_token, left_over_redir) = redir_str.split_at(redir_str.find(char::is_whitespace).unwrap());
            redir_str = &left_over_redir[1..];  // Skip the first char, which is the whitespace
            
            // Get redir file
            let (redir_file, left_over_redir) = redir_str.split_at(redir_str.find(char::is_whitespace).unwrap_or(redir_str.len())); 
            redir_str = left_over_redir;
    
            // Generate a redir_cmd
            let redir_cmd = match redir_token {
                "<" => box Self::new(subcmd, redir_file, FileMode::Read, 0),
                ">" => box Self::new(subcmd, redir_file, FileMode::Write|FileMode::Create, 1),
                c => panic!("unknown redir token: {}", c),
            };

            // Update redir_str and subcmd for next iteration
            subcmd = redir_cmd;
        }

        // Return the result
        (subcmd, redir_str)
    }
}

impl Command for RedirCommand {
    fn run(&self, mut redir: Redir) -> LinkedList<Box<dyn Thread>> {
        // Setup redirection
        let fd = match self.fd {
            0 => &mut redir.stdin,
            1 => &mut redir.stdout,
            2 => &mut redir.stderr,
            n => panic!("fd {} redirection is not intended", n),
        };

        let fd1 = sys_open(&self.file, self.mode).unwrap();

        *fd = fd1;

        // // Run commands
        let result = self.cmd.run(redir);

        // // Cleanup
        // // We are safe to close these fds here because the fdtable is already saved
        // // when sys_spawn_domain returns
        sys_close(fd1).unwrap();
        result
    }
}