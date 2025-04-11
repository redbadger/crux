// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.novi.bincode;

import com.novi.serde.SerializationError;
import com.novi.serde.BinarySerializer;

public class BincodeSerializer extends BinarySerializer {
    public BincodeSerializer() {
        super(Long.MAX_VALUE);
    }

    public void serialize_f32(Float value) throws SerializationError {
        serialize_i32(Integer.valueOf(Float.floatToRawIntBits(value.floatValue())));
    }

    public void serialize_f64(Double value) throws SerializationError {
        serialize_i64(Long.valueOf(Double.doubleToRawLongBits(value.doubleValue())));
    }

    public void serialize_len(long value) throws SerializationError {
        serialize_u64(value);
    }

    public void serialize_variant_index(int value) throws SerializationError {
        serialize_u32(value);
    }

    public void sort_map_entries(int[] offsets) {
        // Not required by the format.
    }
}
