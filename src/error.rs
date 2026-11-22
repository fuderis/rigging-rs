use macron::{Display, Error, From};

#[derive(Debug, Error, Display, From)]
pub enum Error {
    Io(std::io::Error),

    #[cfg(feature = "parser")]
    Parser(crate::parser::error::Error),
}
