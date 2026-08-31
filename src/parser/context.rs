use crate::prelude::*;
use std::str::FromStr;

/// Execution context passed to command handlers.
///
/// Holds all key-value mappings for extracted positional arguments and parsed flags.
#[derive(Debug, Default)]
pub struct Context {
    /// Internal lookup map storing raw argument keys and their extracted string values.
    pub args: HashMap<String, String>,
}

impl Context {
    /// Retrieves a required argument and parses it into the target type `T`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The argument with the specified `key` does not exist in the context.
    /// - The argument value is empty.
    /// - Parsing the raw string value into type `T` via [`FromStr`] fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let port: u16 = ctx.get("port")?;
    /// ```
    pub fn get<T: FromStr>(&self, key: &str) -> Result<T>
    where
        T::Err: std::error::Error + Send + Sync + 'static,
    {
        let raw = self
            .args
            .get(key)
            .ok_or_else(|| format!("Argument '{}' not found", key))?;

        if raw.is_empty() {
            return Err(format!("Argument '{}' is empty", key).into());
        }

        raw.parse::<T>()
            .map_err(|e| format!("Failed to parse argument '{}': {}", key, e).into())
    }

    /// Retrieves an optional argument and parses it into `Option<T>`.
    ///
    /// If the argument is missing or contains an empty string, this method returns `Ok(None)`
    /// without attempting to parse.
    ///
    /// # Errors
    ///
    /// Returns an error if the argument is present and non-empty, but parsing into type `T`
    /// via [`FromStr`] fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let uid: Option<u32> = ctx.get_opt("uid")?;
    /// ```
    pub fn get_opt<T: FromStr>(&self, key: &str) -> Result<Option<T>>
    where
        T::Err: std::error::Error + Send + Sync + 'static,
    {
        match self.args.get(key) {
            None => Ok(None),
            Some(raw) if raw.is_empty() => Ok(None),
            Some(raw) => raw
                .parse::<T>()
                .map(Some)
                .map_err(|e| format!("Failed to parse optional argument '{}': {}", key, e).into()),
        }
    }

    /// Retrieves a raw string slice reference for an argument, if present.
    ///
    /// Unlike [`get`](Self::get), this method does not validate whether the string is empty
    /// and performs no type conversions.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if let Some(path) = ctx.get_str("config") {
    ///     println!("Using config path: {}", path);
    /// }
    /// ```
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.args.get(key).map(|s| s.as_str())
    }

    /// Evaluates whether an argument represents a truthy boolean value.
    ///
    /// Returns `true` if the argument is present and equals `"true"` or `"1"`.
    /// Returns `false` for any other value, empty string, or missing key.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let verbose = ctx.get_bool("verbose");
    /// ```
    pub fn get_bool(&self, key: &str) -> bool {
        self.args
            .get(key)
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
    }
}
