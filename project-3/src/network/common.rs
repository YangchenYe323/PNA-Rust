use serde::{Deserialize, Serialize};

/// A client's Command, which describes what operation client intends to perform
/// on the KvsEngine at the Server end and the argument provided to those operations.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Command {
    /// get the string value of key
    Get {
        /// the string key
        key: String,
    },

    /// set the value of key
    Set {
        /// the string key
        key: String,
        /// the value
        val: String,
    },

    /// remove the value of key
    Remove {
        /// the string key
        key: String,
    },
}

/// Server's Response that corresponds to the previous [Command](crate::Command)
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Response {
    /// flag indicating whether the previous command succeeds or not
    pub success: bool,
    /// the message of the previous command, it carries possible data on success
    /// and error message on failure
    pub message: String,
}

impl Response {
    /// construct a success response
    pub fn success(message: String) -> Self {
        Self {
            success: true,
            message,
        }
    }

    /// construct a failure response
    pub fn failure(message: String) -> Self {
        Self {
            success: false,
            message,
        }
    }
}
