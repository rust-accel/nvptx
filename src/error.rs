use failure;
use std::{io, process};

#[derive(Debug, Clone, Copy)]
pub enum Step {
    Ready,
    Link,
    Build,
    Load,
}

#[derive(Fail, Debug)]
pub enum CompileError {
    #[fail(
        display = "External command {} failed during {:?} step. Return code: {}",
        command,
        step,
        error_code
    )]
    CommandFailure {
        step: Step,
        command: String,
        error_code: i32,
    },
    #[fail(
        display = "External command {} failed during {:?}. Please ensure it is installed.",
        command,
        step,
    )]
    CommandIOFailure {
        step: Step,
        command: String,
        error: io::Error,
    },
    #[fail(
        display = "LLVM Command {} or postfixed by *-6.0 or *-7.0 are not found.",
        command,
    )]
    LLVMCommandNotFound { command: String },
    #[fail(
        display = "Unexpected IO Error during {:?} step: {:?}",
        step,
        error
    )]
    UnexpectedIOError { step: Step, error: io::Error },
}

pub type Result<T> = ::std::result::Result<T, failure::Error>;

pub trait Logging {
    type T;
    fn log(self, step: Step) -> Result<Self::T>;
}

impl<T> Logging for io::Result<T> {
    type T = T;
    fn log(self, step: Step) -> Result<Self::T> {
        self.map_err(|error| CompileError::UnexpectedIOError { step, error }.into())
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
                    }.into())
                } else {
                    Ok(())
                }
            }
            None => Ok(()),
        }
    }
}
