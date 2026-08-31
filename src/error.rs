use crate::prelude::DynError;
use macron::{Display, Error};

/// The argument parsing error
#[derive(Debug, Error, Display)]
pub enum ParseError {
    #[display(fmt = "{0}\n\n{1}")]
    ErrorWithHelp(#[source] DynError, String),

    #[display(fmt = "Unrecognized command '{0}'.")]
    UnknownCommand(String),

    #[display(fmt = "The required flag '--{0}' is missing.")]
    MissingRequiredFlag(String),

    #[display(fmt = "The required argument '<{0}>' is missing.")]
    MissingPositional(String),

    #[display(fmt = "Unexpected argument '{0}'.")]
    UnexpectedArgument(String),

    #[display(fmt = "Unknown flag '{0}'.")]
    UnknownFlag(String),
}
