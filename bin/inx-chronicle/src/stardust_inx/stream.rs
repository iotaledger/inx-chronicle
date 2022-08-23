use chronicle::types::{ledger::OutputWithMetadata, tangle::MilestoneIndex};
use futures::Stream;
use pin_project::pin_project;

use crate::stardust_inx::InxError;

#[derive(Debug)]
pub struct LedgerUpdateRecord {
    pub milestone_index: MilestoneIndex,
    pub outputs: Vec<OutputWithMetadata>,
}

#[pin_project]
pub struct LedgerUpdateStream<S> {
    #[pin]
    inner: S,
    #[pin]
    record: Option<LedgerUpdateRecord>,
}

impl<S> LedgerUpdateStream<S> {
    pub fn new(inner: S) -> Self {
        Self { inner, record: None }
    }
}

impl<S: Stream<Item = Result<bee_inx::LedgerUpdate, bee_inx::Error>>> Stream for LedgerUpdateStream<S> {
    type Item = Result<LedgerUpdateRecord, InxError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        use bee_inx::LedgerUpdate;

        let mut this = self.project();
        Poll::Ready(loop {
            if let Poll::Ready(next) = this.inner.as_mut().poll_next(cx) {
                if let Some(res) = next {
                    match res {
                        Ok(ledger_update) => match ledger_update {
                            LedgerUpdate::Begin(marker) => {
                                // We shouldn't already have a record. If we do, that's bad.
                                if let Some(record) = this.record.as_mut().take() {
                                    break Some(Err(InxError::InvalidLedgerUpdateCount {
                                        received: record.outputs.len(),
                                        expected: record.outputs.capacity(),
                                    }));
                                } else {
                                    this.record.set(Some(LedgerUpdateRecord {
                                        milestone_index: marker.milestone_index.into(),
                                        outputs: Vec::with_capacity(marker.created_count + marker.consumed_count),
                                    }));
                                }
                            }
                            LedgerUpdate::Consumed(consumed) => {
                                if let Some(mut record) = this.record.as_mut().as_pin_mut() {
                                    match OutputWithMetadata::try_from(consumed) {
                                        Ok(consumed) => {
                                            record.outputs.push(consumed);
                                        }
                                        Err(e) => {
                                            break Some(Err(e.into()));
                                        }
                                    }
                                } else {
                                    break Some(Err(InxError::InvalidMilestoneState));
                                }
                            }
                            LedgerUpdate::Created(created) => {
                                if let Some(mut record) = this.record.as_mut().as_pin_mut() {
                                    match OutputWithMetadata::try_from(created) {
                                        Ok(created) => {
                                            record.outputs.push(created);
                                        }
                                        Err(e) => {
                                            break Some(Err(e.into()));
                                        }
                                    }
                                } else {
                                    break Some(Err(InxError::InvalidMilestoneState));
                                }
                            }
                            LedgerUpdate::End(marker) => {
                                if let Some(record) = this.record.as_mut().take() {
                                    if record.outputs.len() != marker.consumed_count + marker.created_count {
                                        break Some(Err(InxError::InvalidLedgerUpdateCount {
                                            received: record.outputs.len(),
                                            expected: marker.consumed_count + marker.created_count,
                                        }));
                                    }
                                    break Some(Ok(record));
                                } else {
                                    break Some(Err(InxError::InvalidMilestoneState));
                                }
                            }
                        },
                        Err(e) => {
                            break Some(Err(e.into()));
                        }
                    }
                } else {
                    // If we were supposed to be in the middle of a milestone, something went wrong.
                    if let Some(record) = this.record.as_mut().take() {
                        break Some(Err(InxError::InvalidLedgerUpdateCount {
                            received: record.outputs.len(),
                            expected: record.outputs.capacity(),
                        }));
                    } else {
                        break None;
                    }
                }
            }
        })
    }
}
