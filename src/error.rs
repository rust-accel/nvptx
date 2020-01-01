use failure::Fail;
use std::{io, process};

#[derive(Debug, Clone, Copy)]
pub enum Step {
    Install,
    Ready,
    Link,
    Build,
    Convert,
    Load,
}

#[derive(Fail, Debug)]
pub enum CompileError {
    #[fail(
        display = "External command {} failed during {:?} step. Return code: {}",
        command, step, error_code
    )]
    CommandFailure {
        step: Step,
        command: String,
        error_code: i32,
    },

    #[fail(
        display = "External command {} failed during {:?}. Please ensure it is installed.",
        command, step
    )]
    CommandIOFailure {
        step: Step,
        command: String,
        error: io::Error,
    },

    #[fail(
        display = "Error during {:?} step: {:?}, error: {:?}",
        step, comment, error
    )]
    OtherError {
        step: Step,
        comment: String,
        error: failure::Error,
    },
}

pub fn err_msg(step: Step, comment: &str) -> CompileError {
    CompileError::OtherError {
        step,
        comment: comment.to_owned(),
        error: failure::err_msg(comment.to_owned()),
    }
}

pub type Result<T> = ::std::result::Result<T, CompileError>;
pub type ResultAny<T> = ::std::result::Result<T, failure::Error>;

pub trait Logging {
    type T;
    fn log_unwrap(self, step: Step) -> Result<Self::T>;
    fn log(self, step: Step, comment: &str) -> Result<Self::T>;
}

impl<T, E: Into<failure::Error>> Logging for ::std::result::Result<T, E> {
    type T = T;

    fn log_unwrap(self, step: Step) -> Result<Self::T> {
        self.log(step, "Unknown IO Error")
    }

    fn log(self, step: Step, comment: &str) -> Result<Self::T> {
        self.map_err(|e| {
            CompileError::OtherError {
                step,
                comment: comment.to_owned(),
                error: e.into(),
            }
            .into()
        })
    }
}

pub trait CheckRun {
    fn check_run(&mut self, step: Step) -> Result<()>;
}

impl CheckRun for process::Command {
    fn check_run(&mut self, step: Step) -> Result<()> {
        let st = self.status().map_err(|error| {
            let command = format!("{:?}", self);
            CompileError::CommandIOFailure {
                step,
                command,
                error,
            }
        })?;
        match st.code() {
            Some(error_code) => {
                if error_code != 0 {
                    let command = format!("{:?}", self);
                    Err(CompileError::CommandFailure {
                        step,
                        command,
                        error_code,
                    }
                    .into())
                } else {
                    Ok(())
                }
            }
            None => Ok(()),
        }
    }
}
