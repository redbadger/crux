// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.novi.serde;

import java.math.BigInteger;

public abstract class BinarySerializer implements Serializer {
    protected MyByteArrayOutputStream output;
    private long containerDepthBudget;

    public BinarySerializer(long maxContainerDepth) {
        output = new BinarySerializer.MyByteArrayOutputStream();
        containerDepthBudget = maxContainerDepth;
    }

    public void increase_container_depth() throws SerializationError {
        if (containerDepthBudget == 0) {
            throw new SerializationError("Exceeded maximum container depth");
        }
        containerDepthBudget -= 1;
    }

    public void decrease_container_depth() {
        containerDepthBudget += 1;
    }

    public void serialize_str(String value) throws SerializationError {
        serialize_bytes(new Bytes(value.getBytes()));
    }

    public void serialize_bytes(Bytes value) throws SerializationError {
        byte[] content = value.content();
        serialize_len(content.length);
        output.write(content, 0, content.length);
    }

    public void serialize_bool(Boolean value) throws SerializationError {
        output.write((value.booleanValue() ? 1 : 0));
    }

    public void serialize_unit(Unit value) throws SerializationError {
    }

    public void serialize_char(Character value) throws SerializationError {
        throw new SerializationError("Not implemented: serialize_char");
    }

    public void serialize_u8(@Unsigned Byte value) throws SerializationError {
        output.write(value.byteValue());
    }

    public void serialize_u16(@Unsigned Short value) throws SerializationError {
        short val = value.shortValue();
        output.write((byte) (val >>> 0));
        output.write((byte) (val >>> 8));
    }

    public void serialize_u32(@Unsigned Integer value) throws SerializationError {
        int val = value.intValue();
        output.write((byte) (val >>> 0));
        output.write((byte) (val >>> 8));
        output.write((byte) (val >>> 16));
        output.write((byte) (val >>> 24));
    }

    public void serialize_u64(@Unsigned Long value) throws SerializationError {
        long val = value.longValue();
        output.write((byte) (val >>> 0));
        output.write((byte) (val >>> 8));
        output.write((byte) (val >>> 16));
        output.write((byte) (val >>> 24));
        output.write((byte) (val >>> 32));
        output.write((byte) (val >>> 40));
        output.write((byte) (val >>> 48));
        output.write((byte) (val >>> 56));
    }

    public void serialize_u128(@Unsigned @Int128 BigInteger value) throws SerializationError {
        if (value.compareTo(BigInteger.ZERO) < 0 || !value.shiftRight(128).equals(BigInteger.ZERO)) {
            throw new java.lang.IllegalArgumentException("Invalid value for an unsigned int128");
        }
        byte[] content = value.toByteArray();
        // BigInteger.toByteArray() may add a most-significant zero
        // byte for signing purpose: ignore it.
        assert content.length <= 16 || content[0] == 0;
        int len = Math.min(content.length, 16);
        // Write content in little-endian order.
        for (int i = 0; i < len; i++) {
            output.write(content[content.length - 1 - i]);
        }
        // Complete with zeros if needed.
        for (int i = len; i < 16; i++) {
            output.write(0);
        }
    }

    public void serialize_i8(Byte value) throws SerializationError {
        serialize_u8(value);
    }

    public void serialize_i16(Short value) throws SerializationError {
        serialize_u16(value);
    }

    public void serialize_i32(Integer value) throws SerializationError {
        serialize_u32(value);
    }

    public void serialize_i64(Long value) throws SerializationError {
        serialize_u64(value);
    }

    public void serialize_i128(@Int128 BigInteger value) throws SerializationError {
        if (value.compareTo(BigInteger.ZERO) >= 0) {
            if (!value.shiftRight(127).equals(BigInteger.ZERO)) {
                throw new java.lang.IllegalArgumentException("Invalid value for a signed int128");
            }
            serialize_u128(value);
        } else {
            if (!value.add(BigInteger.ONE).negate().shiftRight(127).equals(BigInteger.ZERO)) {
                throw new java.lang.IllegalArgumentException("Invalid value for a signed int128");
            }
            serialize_u128(value.add(BigInteger.ONE.shiftLeft(128)));
        }
    }

    public void serialize_option_tag(boolean value) throws SerializationError {
        output.write((value ? (byte) 1 : (byte) 0));
    }

    public int get_buffer_offset() {
        return output.size();
    }

    public byte[] get_bytes() {
        return output.toByteArray();
    }

    // Local extension to provide access to the underlying buffer.
    static public class MyByteArrayOutputStream extends java.io.ByteArrayOutputStream {
        public byte[] getBuffer() {
            return buf;
        }
    }
}
