// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::bson::{document::ValueAccessError, Bson, Document};
use thiserror::Error;

/// Gets values and upcasts if necessary
#[allow(missing_docs)]
pub trait BsonExt: private_bson_ext::SealedBsonExt {
    fn as_string(&self) -> Result<String, ValueAccessError>;

    fn as_u8(&self) -> Result<u8, ValueAccessError>;

    fn as_u16(&self) -> Result<u16, ValueAccessError>;

    fn as_u32(&self) -> Result<u32, ValueAccessError>;

    fn as_u64(&self) -> Result<u64, ValueAccessError>;

    fn to_bytes(self) -> Result<Vec<u8>, ValueAccessError>;

    fn to_array(self) -> Result<Vec<Bson>, ValueAccessError>;

    fn to_document(self) -> Result<Document, ValueAccessError>;
}

impl BsonExt for Bson {
    fn as_string(&self) -> Result<String, ValueAccessError> {
        Ok(match self {
            Bson::Double(i) => i.to_string(),
            Bson::String(i) => i.to_string(),
            Bson::Document(i) => i.to_string(),
            Bson::Boolean(i) => i.to_string(),
            Bson::Null => "null".to_string(),
            Bson::RegularExpression(i) => i.to_string(),
            Bson::JavaScriptCode(i) => i.to_string(),
            Bson::JavaScriptCodeWithScope(i) => i.to_string(),
            Bson::Int32(i) => i.to_string(),
            Bson::Int64(i) => i.to_string(),
            Bson::Timestamp(i) => i.to_string(),
            Bson::Binary(i) => i.to_string(),
            Bson::ObjectId(i) => i.to_string(),
            Bson::DateTime(i) => i.to_string(),
            Bson::Symbol(i) => i.to_string(),
            Bson::Decimal128(i) => i.to_string(),
            Bson::Undefined => "undefined".to_string(),
            _ => return Err(ValueAccessError::UnexpectedType),
        })
    }

    fn as_u8(&self) -> Result<u8, ValueAccessError> {
        Ok(match self {
            Bson::Double(i) => *i as u8,
            Bson::Boolean(i) => *i as u8,
            Bson::Int32(i) => *i as u8,
            Bson::Int64(i) => *i as u8,
            _ => return Err(ValueAccessError::UnexpectedType),
        })
    }

    fn as_u16(&self) -> Result<u16, ValueAccessError> {
        Ok(match self {
            Bson::Double(i) => *i as u16,
            Bson::Boolean(i) => *i as u16,
            Bson::Int32(i) => *i as u16,
            Bson::Int64(i) => *i as u16,
            _ => return Err(ValueAccessError::UnexpectedType),
        })
    }

    fn as_u32(&self) -> Result<u32, ValueAccessError> {
        Ok(match self {
            Bson::Double(i) => *i as u32,
            Bson::Boolean(i) => *i as u32,
            Bson::Int32(i) => *i as u32,
            Bson::Int64(i) => *i as u32,
            Bson::Timestamp(i) => i.time,
            _ => return Err(ValueAccessError::UnexpectedType),
        })
    }

    fn as_u64(&self) -> Result<u64, ValueAccessError> {
        Ok(match self {
            Bson::Double(i) => *i as u64,
            Bson::Boolean(i) => *i as u64,
            Bson::Int32(i) => *i as u64,
            Bson::Int64(i) => *i as u64,
            Bson::Timestamp(i) => i.time as u64,
            _ => return Err(ValueAccessError::UnexpectedType),
        })
    }

    fn to_bytes(self) -> Result<Vec<u8>, ValueAccessError> {
        Ok(match self {
            Bson::Binary(i) => i.bytes,
            _ => return Err(ValueAccessError::UnexpectedType),
        })
    }

    fn to_array(self) -> Result<Vec<Bson>, ValueAccessError> {
        match self {
            Bson::Array(i) => Ok(i),
            _ => Err(ValueAccessError::UnexpectedType),
        }
    }

    fn to_document(self) -> Result<Document, ValueAccessError> {
        match self {
            Bson::Document(i) => Ok(i),
            _ => Err(ValueAccessError::UnexpectedType),
        }
    }
}

mod private_bson_ext {
    use mongodb::bson::Bson;
    pub trait SealedBsonExt {}

    impl SealedBsonExt for Bson {}
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum DocError {
    #[error(transparent)]
    Convert(#[from] mongodb::bson::de::Error),
    #[error("Missing key {0}")]
    MissingKey(String),
    #[error("Value for key {0} is null")]
    NullValue(String),
    #[error(transparent)]
    ValueAccess(#[from] ValueAccessError),
}

#[allow(missing_docs)]
pub trait DocPath: private_doc_path::SealedDocPath {
    fn into_segments(self) -> Vec<String>;
}

impl DocPath for &str {
    fn into_segments(self) -> Vec<String> {
        self.split('.').map(|s| s.to_string()).collect()
    }
}

impl<S: AsRef<str>> DocPath for Vec<S> {
    fn into_segments(self) -> Vec<String> {
        self.iter().map(|s| s.as_ref().to_string()).collect()
    }
}

mod private_doc_path {
    pub trait SealedDocPath {}

    impl SealedDocPath for &str {}

    impl<S: AsRef<str>> SealedDocPath for Vec<S> {}
}

#[allow(missing_docs)]
pub trait DocExt: private_doc_ext::SealedDocExt {
    fn get_bson(&self, path: impl DocPath) -> Result<&Bson, DocError>;

    fn take_bson(&mut self, path: impl DocPath) -> Result<Bson, DocError>;

    fn take_array(&mut self, path: impl DocPath) -> Result<Vec<Bson>, DocError>;

    fn take_bytes(&mut self, path: impl DocPath) -> Result<Vec<u8>, DocError>;

    fn take_document(&mut self, path: impl DocPath) -> Result<Document, DocError>;

    fn get_as_string(&self, path: impl DocPath) -> Result<String, DocError>;

    fn get_as_u8(&self, path: impl DocPath) -> Result<u8, DocError>;

    fn get_as_u16(&self, path: impl DocPath) -> Result<u16, DocError>;

    fn get_as_u32(&self, path: impl DocPath) -> Result<u32, DocError>;

    fn get_as_u64(&self, path: impl DocPath) -> Result<u64, DocError>;
}

impl DocExt for Document {
    fn get_bson(&self, path: impl DocPath) -> Result<&Bson, DocError> {
        let mut doc = self;
        let mut path = path.into_segments();
        if path.is_empty() {
            return Err(DocError::MissingKey("".into()));
        }
        // Unwrap: Totes ok because we just checked that it's not empty.
        let last = path.pop().unwrap();
        for key in path {
            doc = doc.get_document(key)?;
        }
        let bson = self.get(&last).ok_or_else(|| DocError::MissingKey(last.clone()))?;
        match bson {
            Bson::Null => Err(DocError::NullValue(last)),
            _ => Ok(bson),
        }
    }

    fn take_bson(&mut self, path: impl DocPath) -> Result<Bson, DocError> {
        let mut doc = self;
        let mut path = path.into_segments();
        if path.is_empty() {
            return Err(DocError::MissingKey("".into()));
        }
        // Unwrap: Totes ok because we just checked that it's not empty.
        let last = path.pop().unwrap();
        for key in path {
            doc = doc.get_document_mut(key)?;
        }
        let bson = doc.remove(&last).ok_or_else(|| DocError::MissingKey(last.clone()))?;
        match bson {
            Bson::Null => Err(DocError::NullValue(last)),
            _ => Ok(bson),
        }
    }

    fn take_array(&mut self, path: impl DocPath) -> Result<Vec<Bson>, DocError> {
        Ok(self.take_bson(path)?.to_array()?)
    }
    fn take_bytes(&mut self, path: impl DocPath) -> Result<Vec<u8>, DocError> {
        Ok(self.take_bson(path)?.to_bytes()?)
    }

    fn take_document(&mut self, path: impl DocPath) -> Result<Document, DocError> {
        Ok(self.take_bson(path)?.to_document()?)
    }

    fn get_as_string(&self, path: impl DocPath) -> Result<String, DocError> {
        Ok(self.get_bson(path)?.as_string()?)
    }

    fn get_as_u8(&self, path: impl DocPath) -> Result<u8, DocError> {
        Ok(self.get_bson(path)?.as_u8()?)
    }

    fn get_as_u16(&self, path: impl DocPath) -> Result<u16, DocError> {
        Ok(self.get_bson(path)?.as_u16()?)
    }

    fn get_as_u32(&self, path: impl DocPath) -> Result<u32, DocError> {
        Ok(self.get_bson(path)?.as_u32()?)
    }

    fn get_as_u64(&self, path: impl DocPath) -> Result<u64, DocError> {
        Ok(self.get_bson(path)?.as_u64()?)
    }
}

mod private_doc_ext {
    use mongodb::bson::Document;
    pub trait SealedDocExt {}

    impl SealedDocExt for Document {}
}
