use std::fmt;
use std::io;

#[derive(Debug)]
pub enum SystemError {
    MissingEnv(&'static str),
    Io(io::Error),
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEnv(var) => {
                write!(f, "Missing environment variable: {var}")
            }
            Self::Io(err) => write!(f, "IO error: {err}"),
        }
    }
}

impl From<io::Error> for SystemError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
