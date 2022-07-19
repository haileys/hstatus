// The code in this file is based on code taken from:
//   https://github.com/spease/wpa-ctrl-rs
//
// MIT License
//
// Copyright (c) 2017 Sauyon Lee
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

#![deny(missing_docs)]

use std::{io, str};

/// The errors that may occur using `wpactrl`
#[derive(Debug)]
pub enum Error {
    /// Represents all cases of `std::io::Error`.
    Io(io::Error),

    /// Represents a failure to interpret a sequence of u8 as a string slice.
    Utf8ToStr(str::Utf8Error),

    /// Represents a failed `ATTACH` request to wpasupplicant.
    Attach,

    /// Error waiting for a response
    Wait
}

pub type Result<T> = ::std::result::Result<T, Error>;

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::Attach|Self::Wait => None,
            Self::Io(ref source) => Some(source),
            Self::Utf8ToStr(ref source) => Some(source),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::Attach => {
                write!(f, "Failed to attach to wpasupplicant")
            }
            Self::Wait => {
                write!(f, "Unable to wait for response from wpasupplicant")
            }
            Self::Io(ref err) => {
                write!(f, "Failed to execute the specified command: {}", err)
            }
            Self::Utf8ToStr(ref err) => {
                write!(f, "Failed to parse UTF8 to string: {}", err)
            }
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<str::Utf8Error> for Error {
    fn from(err: str::Utf8Error) -> Self {
        Self::Utf8ToStr(err)
    }
}

use std::collections::VecDeque;
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::time::Duration;

const BUF_SIZE: usize = 10_240;
const PATH_DEFAULT_CLIENT: &str = "/tmp";
const PATH_DEFAULT_SERVER: &str = "/var/run/wpa_supplicant/wlan0";

/// Builder object used to construct a [`Client`] session
#[derive(Default)]
pub struct ClientBuilder {
    cli_path: Option<PathBuf>,
    ctrl_path: Option<PathBuf>,
}

impl ClientBuilder {
    /// A path-like object for the `wpa_supplicant` / `hostapd` UNIX domain sockets
    ///
    /// # Examples
    ///
    /// ```
    /// use wpactrl::Client;
    /// let wpa = Client::builder()
    ///             .ctrl_path("/var/run/wpa_supplicant/wlan0")
    ///             .open()
    ///             .unwrap();
    /// ```
    #[must_use]
    pub fn ctrl_path<I, P>(mut self, ctrl_path: I) -> Self
    where
        I: Into<Option<P>>,
        P: AsRef<Path> + Sized,
        PathBuf: From<P>,
    {
        self.ctrl_path = ctrl_path.into().map(PathBuf::from);
        self
    }

    /// Open a control interface to `wpa_supplicant` / `hostapd`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wpactrl::Client;
    /// let wpa = Client::builder().open().unwrap();
    /// ```
    /// # Errors
    ///
    /// * [[`Error::Io`]] - Low-level I/O error
    pub fn open(self) -> Result<Client> {
        let mut counter = 0;
        loop {
            counter += 1;
            let bind_filename = format!("wpa_ctrl_{}-{}", std::process::id(), counter);
            let bind_filepath = self
                .cli_path
                .as_deref()
                .unwrap_or_else(|| Path::new(PATH_DEFAULT_CLIENT))
                .join(bind_filename);
            match UnixDatagram::bind(&bind_filepath) {
                Ok(socket) => {
                    socket.connect(self.ctrl_path.unwrap_or_else(|| PATH_DEFAULT_SERVER.into()))?;
                    socket.set_nonblocking(true)?;
                    return Ok(Client(ClientInternal {
                        buffer: [0; BUF_SIZE],
                        handle: socket,
                        filepath: bind_filepath,
                    }));
                }
                Err(ref e) if counter < 2 && e.kind() == std::io::ErrorKind::AddrInUse => {
                    std::fs::remove_file(bind_filepath)?;
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
        }
    }
}

struct ClientInternal {
    buffer: [u8; BUF_SIZE],
    handle: UnixDatagram,
    filepath: PathBuf,
}

fn select(fd: RawFd, duration: Duration) -> Result<bool> {
    let r = unsafe {
        let mut raw_fd_set = {
            let mut raw_fd_set = std::mem::MaybeUninit::<libc::fd_set>::uninit();
            libc::FD_ZERO(raw_fd_set.as_mut_ptr());
            raw_fd_set.assume_init()
        };
        libc::FD_SET(fd, &mut raw_fd_set);
        libc::select(
            fd + 1,
            &mut raw_fd_set,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut libc::timeval {
                tv_sec: duration.as_secs().try_into().unwrap(),
                tv_usec: duration.subsec_micros().try_into().unwrap(),
            },
        )
    };

    if r >= 0 {
        Ok(r > 0)
    } else {
        Err(Error::Wait)
    }
}

impl ClientInternal {
    /// Check if any messages are available
    pub fn pending(&mut self) -> Result<bool> {
        select(self.handle.as_raw_fd(), Duration::from_secs(0))
    }

    /// Receive a message
    pub fn recv(&mut self) -> Result<Option<String>> {
        if self.pending()? {
            let buf_len = self.handle.recv(&mut self.buffer)?;
            std::str::from_utf8(&self.buffer[0..buf_len])
                .map(|s| Some(s.to_owned()))
                .map_err(std::convert::Into::into)
        } else {
            Ok(None)
        }
    }

    fn send_request(&mut self, cmd: &str) -> Result<()> {
        self.handle.send(cmd.as_bytes())?;
        Ok(())
    }

    /// Send a command to `wpa_supplicant` / `hostapd`.
    fn request<F: FnMut(&str)>(&mut self, cmd: &str, mut cb: F) -> Result<String> {
        self.send_request(cmd)?;
        loop {
            select(self.handle.as_raw_fd(), Duration::from_secs(10))?;
            match self.handle.recv(&mut self.buffer) {
                Ok(len) => {
                    let s = std::str::from_utf8(&self.buffer[0..len])?;
                    if s.starts_with('<') {
                        cb(s);
                    } else {
                        return Ok(s.to_owned());
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }
}

impl AsRawFd for ClientInternal {
    fn as_raw_fd(&self) -> RawFd {
        self.handle.as_raw_fd()
    }
}

impl Drop for ClientInternal {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.filepath) {
            eprintln!("wpactrl: Unable to unlink {:?}", e);
        }
    }
}

/// A connection to `wpa_supplicant` / `hostapd`
pub struct Client(ClientInternal);

impl Client {
    /// Creates a builder for a `wpa_supplicant` / `hostapd` connection
    ///
    /// # Examples
    ///
    /// ```
    /// let wpa = wpactrl::Client::builder().open().unwrap();
    /// ```
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Register as an event monitor for control interface messages
    ///
    /// # Examples
    ///
    /// ```
    /// let mut wpa = wpactrl::Client::builder().open().unwrap();
    /// let wpa_attached = wpa.attach().unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// * [`Error::Attach`] - Unexpected (non-OK) response
    /// * [`Error::Io`] - Low-level I/O error
    /// * [`Error::Utf8ToStr`] - Corrupted message or message with non-UTF8 characters
    /// * [`Error::Wait`] - Failed to wait on underlying Unix socket
    pub fn attach(mut self) -> Result<ClientAttached> {
        // FIXME: None closure would be better
        if self.0.request("ATTACH", |_: &str| ())? == "OK\n" {
            Ok(ClientAttached(self.0, VecDeque::new()))
        } else {
            Err(Error::Attach)
        }
    }
}

impl AsRawFd for Client {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

/// A connection to `wpa_supplicant` / `hostapd` that receives status messages
pub struct ClientAttached(ClientInternal, VecDeque<String>);

impl ClientAttached {
    /// Receive the next control interface message.
    ///
    /// Note that multiple control interface messages can be pending;
    /// call this function repeatedly until it returns None to get all of them.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut wpa = wpactrl::Client::builder().open().unwrap().attach().unwrap();
    /// assert_eq!(wpa.recv().unwrap(), None);
    /// ```
    ///
    /// # Errors
    ///
    /// * [`Error::Io`] - Low-level I/O error
    /// * [`Error::Utf8ToStr`] - Corrupted message or message with non-UTF8 characters
    /// * [`Error::Wait`] - Failed to wait on underlying Unix socket
    pub fn recv(&mut self) -> Result<Option<String>> {
        if let Some(s) = self.1.pop_back() {
            Ok(Some(s))
        } else {
            self.0.recv()
        }
    }

    pub fn send_request(&mut self, cmd: &str) -> Result<()> {
        self.0.send_request(cmd)
    }
}

impl AsRawFd for ClientAttached {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

#[cfg(test)]
mod test {
    use serial_test::serial;
    use super::*;

    fn wpa_ctrl() -> Client {
        Client::builder().open().unwrap()
    }

    #[test]
    #[serial]
    fn attach() {
        wpa_ctrl()
            .attach()
            .unwrap()
            .detach()
            .unwrap()
            .attach()
            .unwrap()
            .detach()
            .unwrap();
    }

    #[test]
    #[serial]
    fn detach() {
        let wpa = wpa_ctrl().attach().unwrap();
        wpa.detach().unwrap();
    }

    #[test]
    #[serial]
    fn builder() {
        wpa_ctrl();
    }

    #[test]
    #[serial]
    fn request() {
        let mut wpa = wpa_ctrl();
        assert_eq!(wpa.request("PING").unwrap(), "PONG\n");
        let mut wpa_attached = wpa.attach().unwrap();
        // FIXME: This may not trigger the callback
        assert_eq!(wpa_attached.request("PING").unwrap(), "PONG\n");
    }

    #[test]
    #[serial]
    fn recv() {
        let mut wpa = wpa_ctrl().attach().unwrap();
        assert_eq!(wpa.recv().unwrap(), None);
        assert_eq!(wpa.request("SCAN").unwrap(), "OK\n");
        loop {
            match wpa.recv().unwrap() {
                Some(s) => {
                    assert_eq!(&s[3..], "CTRL-EVENT-SCAN-STARTED ");
                    break;
                }
                None => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
        wpa.detach().unwrap();
    }
}
