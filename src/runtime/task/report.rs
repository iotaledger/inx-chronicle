// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::{error::TaskError, Task};

/// An task exit report.
#[derive(Debug)]
pub enum TaskReport<T: Task> {
    /// Task exited successfully.
    Success(TaskSuccessReport<T>),
    /// Task exited with an error.
    Error(TaskErrorReport<T>),
}

impl<T: Task> TaskReport<T> {
    /// Gets the task.
    pub fn task(&self) -> &T {
        match self {
            TaskReport::Success(success) => success.task(),
            TaskReport::Error(error) => error.task(),
        }
    }

    /// Takes the task, consuming the report.
    pub fn take_task(self) -> T {
        match self {
            TaskReport::Success(success) => success.take_task(),
            TaskReport::Error(error) => error.take_task(),
        }
    }

    /// Gets the error, if any.
    pub fn error(&self) -> Option<&TaskError<T>> {
        match self {
            TaskReport::Success(_) => None,
            TaskReport::Error(error) => Some(error.error()),
        }
    }

    /// Takes the error, if any.
    pub fn take_error(self) -> Option<TaskError<T>> {
        match self {
            TaskReport::Success(_) => None,
            TaskReport::Error(error) => Some(error.take_error()),
        }
    }
}

/// A report that an task finished running with an error
#[derive(Debug)]
pub struct TaskSuccessReport<T: Task> {
    /// The task's external state when it finished running
    pub task: T,
}

impl<T: Task> TaskSuccessReport<T> {
    pub(crate) fn new(task: T) -> Self {
        Self { task }
    }

    /// Gets the task.
    pub fn task(&self) -> &T {
        &self.task
    }

    /// Takes the task, consuming the report.
    pub fn take_task(self) -> T {
        self.task
    }
}

/// A report that an task finished running with an error.
#[derive(Debug)]
pub struct TaskErrorReport<T: Task> {
    /// The task's external state when it finished running.
    pub task: T,
    /// The error that occurred
    pub error: TaskError<T>,
}

impl<T: Task> TaskErrorReport<T> {
    pub(crate) fn new(task: T, error: TaskError<T>) -> Self {
        Self { task, error }
    }

    /// Gets the task.
    pub fn task(&self) -> &T {
        &self.task
    }

    /// Takes the task, consuming the report.
    pub fn take_task(self) -> T {
        self.task
    }

    /// Gets the error that occurred.
    pub fn error(&self) -> &TaskError<T> {
        &self.error
    }

    /// Takes the error that occurred.
    pub fn take_error(self) -> TaskError<T> {
        self.error
    }
}
