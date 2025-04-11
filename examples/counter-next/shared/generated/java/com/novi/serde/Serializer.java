// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.novi.serde;

import java.math.BigInteger;

public interface Serializer {
    void serialize_str(String value) throws SerializationError;

    void serialize_bytes(Bytes value) throws SerializationError;

    void serialize_bool(Boolean value) throws SerializationError;

    void serialize_unit(Unit value) throws SerializationError;

    void serialize_char(Character value) throws SerializationError;

    void serialize_f32(Float value) throws SerializationError;

    void serialize_f64(Double value) throws SerializationError;

    void serialize_u8(@Unsigned Byte value) throws SerializationError;

    void serialize_u16(@Unsigned Short value) throws SerializationError;

    void serialize_u32(@Unsigned Integer value) throws SerializationError;

    void serialize_u64(@Unsigned Long value) throws SerializationError;

    void serialize_u128(@Unsigned @Int128 BigInteger value) throws SerializationError;

    void serialize_i8(Byte value) throws SerializationError;

    void serialize_i16(Short value) throws SerializationError;

    void serialize_i32(Integer value) throws SerializationError;

    void serialize_i64(Long value) throws SerializationError;

    void serialize_i128(@Int128 BigInteger value) throws SerializationError;

    void serialize_len(long value) throws SerializationError;

    void serialize_variant_index(int value) throws SerializationError;

    void serialize_option_tag(boolean value) throws SerializationError;

    void increase_container_depth() throws SerializationError;

    void decrease_container_depth();

    int get_buffer_offset();

    void sort_map_entries(int[] offsets);

    byte[] get_bytes();
}
