// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use mongodb::bson::{ser::Error, Array, Bson, Document, Serializer as BsonSerializer};
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple, SerializeTupleStruct,
        SerializeTupleVariant,
    },
    Deserialize, Serialize,
};

use crate::BsonExt;

/// Converts a value into a [`Bson`] using custom serialization to avoid data loss.
pub fn to_bson<T: ?Sized>(value: &T) -> Result<Bson, Error>
where
    T: Serialize,
{
    let ser = Serializer(BsonSerializer::new());
    value.serialize(ser)
}

/// Converts a value into a [`Bson`] using custom serialization to avoid data loss.
pub fn to_document<T: ?Sized>(value: &T) -> Result<Document, Error>
where
    T: Serialize,
{
    let ser = Serializer(BsonSerializer::new());
    value.serialize(ser).map(|bson| bson.to_document().unwrap())
}

/// Compatability type for u64 reprentation in BSON.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct U64 {
    hi: u8,
    lo: i64,
}

impl From<u64> for U64 {
    fn from(value: u64) -> Self {
        // Panic: both `unwrap` calls are optimized away due to the fact that the bytes that
        // could be truncated are actually zero.
        Self {
            hi: (value >> 56).try_into().unwrap(),
            lo: ((value << 8) >> 8).try_into().unwrap(),
        }
    }
}

impl From<U64> for u64 {
    fn from(v: U64) -> Self {
        // Using `as` is fine here because `From<u64>` guarantees that `lo` is non-negative.
        (u64::from(v.hi) << 56) | (v.lo as u64)
    }
}

#[repr(transparent)]
struct Serializer(BsonSerializer);

impl serde::Serializer for Serializer {
    type Ok = <BsonSerializer as serde::Serializer>::Ok;

    type Error = <BsonSerializer as serde::Serializer>::Error;

    type SerializeSeq = ArraySerializer;

    type SerializeTuple = ArraySerializer;

    type SerializeTupleStruct = ArraySerializer;

    type SerializeTupleVariant = TupleVariantSerializer;

    type SerializeMap = MapSerializer;

    type SerializeStruct = StructSerializer;

    type SerializeStructVariant = StructVariantSerializer;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_bool(v)
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_i8(v)
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_i16(v)
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_i32(v)
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_i64(v)
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_u8(v)
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_u16(v)
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_u32(v)
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        U64::from(v).serialize(self)
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_f32(v)
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_f64(v)
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_char(v)
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_str(v)
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_bytes(v)
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_none()
    }

    #[inline]
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_unit()
    }

    #[inline]
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_unit_struct(name)
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_unit_variant(name, variant_index, variant)
    }

    #[inline]
    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        let mut newtype_variant = Document::new();
        newtype_variant.insert(variant, to_bson(value)?);
        Ok(newtype_variant.into())
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ArraySerializer {
            inner: Array::with_capacity(len.unwrap_or(0)),
        })
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(ArraySerializer {
            inner: Array::with_capacity(len),
        })
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(ArraySerializer {
            inner: Array::with_capacity(len),
        })
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariantSerializer {
            inner: Array::with_capacity(len),
            name: variant,
        })
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            inner: Document::new(),
            next_key: None,
        })
    }

    #[inline]
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSerializer { inner: Document::new() })
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariantSerializer {
            name: variant,
            inner: Document::new(),
        })
    }
}

struct ArraySerializer {
    inner: Array,
}

impl SerializeSeq for ArraySerializer {
    type Ok = Bson;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.inner.push(to_bson(value)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Bson::Array(self.inner))
    }
}

impl SerializeTuple for ArraySerializer {
    type Ok = Bson;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.inner.push(to_bson(value)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Bson::Array(self.inner))
    }
}

impl SerializeTupleStruct for ArraySerializer {
    type Ok = Bson;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.inner.push(to_bson(value)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Bson::Array(self.inner))
    }
}

struct TupleVariantSerializer {
    inner: Array,
    name: &'static str,
}

impl SerializeTupleVariant for TupleVariantSerializer {
    type Ok = Bson;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.inner.push(to_bson(value)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut tuple_variant = Document::new();
        tuple_variant.insert(self.name, self.inner);
        Ok(tuple_variant.into())
    }
}

struct MapSerializer {
    inner: Document,
    next_key: Option<String>,
}

impl SerializeMap for MapSerializer {
    type Ok = Bson;
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.next_key = match to_bson(&key)? {
            Bson::String(s) => Some(s),
            other => return Err(Error::InvalidDocumentKey(other)),
        };
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let key = self.next_key.take().unwrap_or_default();
        self.inner.insert(key, to_bson(&value)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Bson::from(self.inner))
    }
}

struct StructSerializer {
    inner: Document,
}

impl SerializeStruct for StructSerializer {
    type Ok = Bson;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
        self.inner.insert(key, to_bson(value)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Bson::from(self.inner))
    }
}

struct StructVariantSerializer {
    inner: Document,
    name: &'static str,
}

impl SerializeStructVariant for StructVariantSerializer {
    type Ok = Bson;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
        self.inner.insert(key, to_bson(value)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let var = Bson::from(self.inner);

        let mut struct_variant = Document::new();
        struct_variant.insert(self.name, var);

        Ok(Bson::Document(struct_variant))
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::bson;

    use super::{to_bson, U64};

    const V1: u64 = 0xfedcba9876543210;
    const V2: u64 = 0x0123456789abcdef;

    #[test]
    fn u64_conversion() {
        assert_eq!(
            U64::from(V1),
            U64 {
                hi: 0xfe,
                lo: 0xdcba9876543210
            },
        );

        assert_eq!(
            U64::from(V2),
            U64 {
                hi: 0x01,
                lo: 0x23456789abcdef,
            },
        );
    }

    #[test]
    fn serializer() {
        #[derive(serde::Serialize)]
        struct Foo {
            bar: Bar,
            baz: Option<Baz>,
        }

        #[derive(serde::Serialize)]
        enum Bar {
            A(u64),
        }

        #[derive(serde::Serialize)]
        struct Baz(u64);

        let bson = super::to_bson(&Foo {
            bar: Bar::A(V1),
            baz: Some(Baz(V2)),
        })
        .unwrap();

        assert_eq!(
            bson!({ "bar": { "A": to_bson(&U64::from(V1)).unwrap() }, "baz" : to_bson(&U64::from(V2)).unwrap()}),
            bson
        );
    }
}
