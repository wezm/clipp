//! implements different clipboard types
use std::{
    io::{self, Read, Write},
    process::{Command, Stdio},
};

pub trait Clipboard {
    fn copy(text: &str) -> io::Result<()>;
    fn paste() -> io::Result<String>;
}

macro_rules! c {
    ($p:ident $($args:ident)+) => {
        Command::new(stringify!($p)).args([$(stringify!($args),)+])
    };
    ($p:literal) => {
        Command::new($p)
    };
    ($p:literal $($args:literal)+) => {
        Command::new($p).args([$($args,)+])

    }
}

trait Eat {
    fn eat(&mut self) -> io::Result<String>;
}

impl Eat for Command {
    fn eat(&mut self) -> io::Result<String> {
        let mut s = String::new();
        self.stdout(Stdio::piped())
            .spawn()?
            .stdout
            .take()
            .expect("stdout")
            .read_to_string(&mut s)?;
        Ok(s)
    }
}

trait Put {
    fn put(&mut self, s: impl AsRef<[u8]>) -> io::Result<()>;
}

impl Put for Command {
    fn put(&mut self, s: impl AsRef<[u8]>) -> io::Result<()> {
        let mut ch = self.stdin(Stdio::piped()).spawn()?;
        ch.stdin.take().expect("stdin").write_all(s.as_ref())?;
        ch.wait()?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub struct PbCopy {}
#[cfg(target_os = "macos")]
impl Clipboard for PbCopy {
    fn copy(text: &str) -> io::Result<()> {
        c!(pbcopy w).put(text)
    }

    fn paste() -> io::Result<String> {
        c!(pbcopy r).eat()
    }
}

pub struct XClip {}
impl Clipboard for XClip {
    fn copy(text: &str) -> io::Result<()> {
        c!("xclip" "-selection" "c").put(text)
    }

    fn paste() -> io::Result<String> {
        c!("xclip" "-selection" "c" "-o") // xclip is complainy
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .eat() // If stdout is nulled does this work?
    }
}

pub struct XSel {}
impl Clipboard for XSel {
    fn copy(text: &str) -> io::Result<()> {
        c!("xsel" "-b" "-i").put(text)
    }

    fn paste() -> io::Result<String> {
        c!("xsel" "-b" "-o").eat()
    }
}

struct Wayland {}
impl Clipboard for Wayland {
    fn copy(text: &str) -> io::Result<()> {
        match text {
            "" => c!("wl-copy" "-p" "--clear")
                .status()?
                .success()
                .then_some(())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        String::from("wl-copy was not successful"),
                    )
                }),
            s => c!("wl-copy" "-p").put(s),
        }
    }

    fn paste() -> io::Result<String> {
        c!("wl-paste" "-n" "-p").eat()
    }
}

struct Klipper {}
impl Clipboard for Klipper {
    fn copy(text: &str) -> io::Result<()> {
        c!("qdbus" "org.kde.klipper" "/klipper" "setClipboardContents").arg(text);
        Ok(())
    }

    fn paste() -> io::Result<String> {
        let mut s = c!("qdbus" "org.kde.klipper" "/klipper" "getClipboardContents").eat()?;
        assert!(s.ends_with('\n'));
        s.truncate(s.len() - 1);
        Ok(s)
    }
}

#[cfg(target_family = "windows")]
struct Windows {}
#[cfg(target_family = "windows")]
impl Clipboard for Windows {
    fn copy(text: &str) -> io::Result<()> {
        clipboard_win::set_clipboard_string(text)?
    }

    fn paste() -> io::Result<String> {
        clipboard_win::get_clipboard_string()?
    }
}

struct Wsl {}

impl Clipboard for Wsl {
    fn copy(text: &str) -> io::Result<()> {
        c!("clip.exe").put(text)
    }

    fn paste() -> io::Result<String> {
        let mut s = c!("powershell.exe" "-noprofile" "-command" "Get-Clipboard").eat()?;
        s.truncate(s.len() - 2); // \r\n
        Ok(s)
    }
}

pub type Board = (
    for<'a> fn(&'a str) -> io::Result<()>,
    fn() -> io::Result<String>,
);

fn get<T: Clipboard>() -> Board {
    (T::copy, T::paste)
}

fn has(c: &str) -> bool {
    c!("which")
        .arg(c)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .map_or(false, |status| status.success())
}

fn wsl() -> bool {
std::fs::read_to_string("/proc/version").map_or(false, |s| 
        s.to_lowercase().contains("microsoft")
)
}

pub fn provide() -> io::Result<Board> {
    #[cfg(target_family = "windows")]
    return get::<Windows>();
    #[cfg(target_os = "macos")]
    return get::<PbCopy>();

    if wsl() {
        return Ok(get::<Wsl>());
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && has("wl-copy") {
        Ok(get::<Wayland>())
    } else if has("xsel") {
        Ok(get::<XSel>())
    } else if has("xclip") {
        Ok(get::<XClip>())
    } else if has("klipper") && has("qdbus") {
        Ok(get::<Klipper>())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, String::from("no clipboard provided available")))
    }
}

#[test]
fn test() {
    macro_rules! test {
        ($clipboard:ty) => {
            <$clipboard>::copy("text");
            assert_eq!(<$clipboard>::paste().unwrap(), "text");
            <$clipboard>::copy("");
        };
    }
    #[cfg(target_os = "macos")]
    test!(PbCopy);
    #[cfg(target_os = "linux")]
    test!(XClip);
    #[cfg(target_os = "linux")]
    test!(XSel);
    #[cfg(target_os = "linux")]
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        test!(Wayland);
    }
    #[cfg(target_os = "linux")]
    test!(Klipper);
    #[cfg(target_family = "windows")]
    test!(Windows);
    if wsl() {
        #[cfg(target_os = "linux")]
        test!(Wsl);
    }
}
