// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use super::Task;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum TaskError<T: Task> {
    #[error("Task aborted")]
    Aborted,
    #[error("Task panicked")]
    Panic,
    #[error("Task error: {0}")]
    Result(T::Error),
}
