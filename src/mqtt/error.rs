// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use rumqttc::OptionError;
use thiserror::Error;

#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum MqttError {
    #[error(transparent)]
    Options(#[from] OptionError),
}
