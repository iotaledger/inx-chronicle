// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::{error::TaskError, Task};

/// A task exit report.
pub struct TaskReport<T: Task> {
    /// The task's state when it finished running.
    pub task: T,
    /// The error that occurred, if any.
    pub error: Option<TaskError<T>>,
}
