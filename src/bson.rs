// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;

/// FIXME: docs
pub fn to_bson<T: ?Sized>(value: &T) -> Result<bson::Bson, bson::ser::Error>
where
    T: Serialize,
{
    let ser = BsonSerializer(bson::Serializer::new());
    value.serialize(ser)
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct U64 {
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

impl Into<u64> for U64 {
    fn into(self) -> u64 {
        // Using `as` is fine here because `From<u64>` guarantees that `lo` is non-negative.
        (u64::from(self.hi) << 56) | (self.lo as u64)
    }
}

#[repr(transparent)]
struct BsonSerializer(bson::Serializer);

impl serde::Serializer for BsonSerializer {
    type Ok = <bson::Serializer as serde::Serializer>::Ok;

    type Error = <bson::Serializer as serde::Serializer>::Error;

    type SerializeSeq = <bson::Serializer as serde::Serializer>::SerializeSeq;

    type SerializeTuple = <bson::Serializer as serde::Serializer>::SerializeTuple;

    type SerializeTupleStruct = <bson::Serializer as serde::Serializer>::SerializeTupleStruct;

    type SerializeTupleVariant = <bson::Serializer as serde::Serializer>::SerializeTupleVariant;

    type SerializeMap = <bson::Serializer as serde::Serializer>::SerializeMap;

    type SerializeStruct = <bson::Serializer as serde::Serializer>::SerializeStruct;

    type SerializeStructVariant = <bson::Serializer as serde::Serializer>::SerializeStructVariant;

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
        self.0.serialize_some(value)
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
    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        self.0.serialize_newtype_struct(name, value)
    }

    #[inline]
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        self.0.serialize_newtype_variant(name, variant_index, variant, value)
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.0.serialize_seq(len)
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.0.serialize_tuple(len)
    }

    #[inline]
    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.0.serialize_tuple_struct(name, len)
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.0.serialize_tuple_variant(name, variant_index, variant, len)
    }

    #[inline]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.0.serialize_map(len)
    }

    #[inline]
    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        self.0.serialize_struct(name, len)
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.0.serialize_struct_variant(name, variant_index, variant, len)
    }
}

#[cfg(test)]
mod tests {
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
            baz: Baz,
        }

        #[derive(serde::Serialize)]
        enum Bar {
            A(u64),
        }

        #[derive(serde::Serialize)]
        struct Baz(u64);

        let bson = super::to_bson(&Foo {
            bar: Bar::A(V1),
            baz: Baz(V2),
        })
        .unwrap();

        assert_eq!(
            bson::bson!({ "bar": { "A": to_bson(&U64::from(V1)).unwrap() }, "baz" : to_bson(&U64::from(V2)).unwrap()}),
            bson
        );
    }
}
