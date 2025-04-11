// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.novi.serde;

import java.math.BigInteger;

public interface Deserializer {
    String deserialize_str() throws DeserializationError;

    Bytes deserialize_bytes() throws DeserializationError;

    Boolean deserialize_bool() throws DeserializationError;

    Unit deserialize_unit() throws DeserializationError;

    Character deserialize_char() throws DeserializationError;

    Float deserialize_f32() throws DeserializationError;

    Double deserialize_f64() throws DeserializationError;

    @Unsigned Byte deserialize_u8() throws DeserializationError;

    @Unsigned Short deserialize_u16() throws DeserializationError;

    @Unsigned Integer deserialize_u32() throws DeserializationError;

    @Unsigned Long deserialize_u64() throws DeserializationError;

    @Unsigned @Int128 BigInteger deserialize_u128() throws DeserializationError;

    Byte deserialize_i8() throws DeserializationError;

    Short deserialize_i16() throws DeserializationError;

    Integer deserialize_i32() throws DeserializationError;

    Long deserialize_i64() throws DeserializationError;

    @Int128 BigInteger deserialize_i128() throws DeserializationError;

    long deserialize_len() throws DeserializationError;

    int deserialize_variant_index() throws DeserializationError;

    boolean deserialize_option_tag() throws DeserializationError;

    void increase_container_depth() throws DeserializationError;

    void decrease_container_depth();

    int get_buffer_offset();

    void check_that_key_slices_are_increasing(Slice key1, Slice key2) throws DeserializationError;
}
