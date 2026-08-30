use anyhow::Error;
use std::fmt;

#[derive(Debug)]
pub struct UserError {
    message: String,
}

impl UserError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for UserError {}

pub fn user_error(message: impl Into<String>) -> Error {
    UserError::new(message).into()
}

pub fn exit_code(error: &Error) -> u8 {
    if error.downcast_ref::<UserError>().is_some() {
        1
    } else {
        2
    }
}

pub fn format_error(error: &Error) -> String {
    if error.downcast_ref::<UserError>().is_some() {
        return error.to_string();
    }

    let mut chain = error.chain();
    let mut message = match chain.next() {
        Some(cause) => cause.to_string(),
        None => return String::from("internal error"),
    };

    for cause in chain {
        let cause = cause.to_string();
        if !cause.is_empty() && cause != message {
            message.push_str(": ");
            message.push_str(&cause);
        }
    }

    message
}
