use std::os::fd::FromRawFd;
use std::os::unix::net::{UnixListener, UnixStream};

#[derive(Debug, Clone)]
pub enum IpcCommand {
    Launcher,
    Calculator(Option<String>),
    Clipboard,
    WindowManager,
    System,
    Snippets,
    Quit,
}

impl IpcCommand {
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        match line {
            "launcher" => Some(IpcCommand::Launcher),
            "calculator" => Some(IpcCommand::Calculator(None)),
            "clipboard" => Some(IpcCommand::Clipboard),
            "wm" | "window-manager" | "window_manager" => Some(IpcCommand::WindowManager),
            "system" | "sys" | "system-commands" | "system_commands" => Some(IpcCommand::System),
            "snippets" | "snippet" | "snip" | "S" => Some(IpcCommand::Snippets),
            "quit" => Some(IpcCommand::Quit),
            _ if line.starts_with("calculator:") => {
                Some(IpcCommand::Calculator(Some(line["calculator:".len()..].to_string())))
            }
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            IpcCommand::Launcher => "launcher\n".to_string(),
            IpcCommand::Calculator(None) => "calculator\n".to_string(),
            IpcCommand::Calculator(Some(expr)) => format!("calculator:{}\n", expr),
            IpcCommand::Clipboard => "clipboard\n".to_string(),
            IpcCommand::WindowManager => "wm\n".to_string(),
            IpcCommand::System => "system\n".to_string(),
            IpcCommand::Snippets => "snippets\n".to_string(),
            IpcCommand::Quit => "quit\n".to_string(),
        }
    }

    pub fn send(&self, socket: &mut UnixStream) -> std::io::Result<()> {
        use std::io::Write;
        socket.write_all(self.to_string().as_bytes())
    }
}

fn fill_abstract_addr(name: &str, addr: &mut libc::sockaddr_un) -> libc::socklen_t {
    let name_bytes = name.as_bytes();
    addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
    let path_bytes: Vec<u8> = std::iter::once(0).chain(name_bytes.iter().copied()).collect();
    let path_len = path_bytes.len();
    unsafe {
        std::ptr::copy_nonoverlapping(
            path_bytes.as_ptr() as *const libc::c_char,
            addr.sun_path.as_mut_ptr(),
            path_len,
        );
    }
    (std::mem::offset_of!(libc::sockaddr_un, sun_path) + path_len) as libc::socklen_t
}

/// Bind an abstract Unix domain socket with the given name.
/// The name should NOT include the leading null byte.
pub fn bind_abstract(socket_name: &str) -> std::io::Result<UnixListener> {
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }

    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    let addr_len = fill_abstract_addr(socket_name, &mut addr);

    let ret = unsafe { libc::bind(fd, &addr as *const _ as *const libc::sockaddr, addr_len) };
    if ret < 0 {
        let err = std::io::Error::last_os_error();
        unsafe { libc::close(fd); }
        return Err(err);
    }

    unsafe { libc::listen(fd, 128); }
    let listener = unsafe { UnixListener::from_raw_fd(fd) };
    listener.set_nonblocking(true)?;
    Ok(listener)
}

/// Connect to an abstract Unix domain socket.
pub fn connect_abstract(socket_name: &str) -> std::io::Result<UnixStream> {
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }

    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    let addr_len = fill_abstract_addr(socket_name, &mut addr);

    let ret = unsafe { libc::connect(fd, &addr as *const _ as *const libc::sockaddr, addr_len) };
    if ret < 0 {
        let err = std::io::Error::last_os_error();
        unsafe { libc::close(fd); }
        return Err(err);
    }

    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    Ok(stream)
}

/// Feed raw bytes into a buffer and extract any complete commands.
/// Partial data remains in `buf` for the next call.
pub fn read_commands_from_buf(buf: &mut Vec<u8>, data: &[u8]) -> Vec<IpcCommand> {
    buf.extend_from_slice(data);
    let mut commands = Vec::new();
    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = buf.drain(..=pos).collect();
        if let Ok(s) = String::from_utf8(line) {
            if let Some(cmd) = IpcCommand::parse(&s) {
                commands.push(cmd);
            }
        }
    }
    commands
}
