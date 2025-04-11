// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.novi.serde;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.CharsetDecoder;
import java.nio.charset.StandardCharsets;
import java.nio.charset.CharacterCodingException;
import java.math.BigInteger;

public abstract class BinaryDeserializer implements Deserializer {
    protected ByteBuffer input;
    private long containerDepthBudget;

    public BinaryDeserializer(byte[] input, long maxContainerDepth) {
        this.input = ByteBuffer.wrap(input);
        this.input.order(ByteOrder.LITTLE_ENDIAN);
        containerDepthBudget = maxContainerDepth;
    }

    public void increase_container_depth() throws DeserializationError {
        if (containerDepthBudget == 0) {
            throw new DeserializationError("Exceeded maximum container depth");
        }
        containerDepthBudget -= 1;
    }

    public void decrease_container_depth() {
        containerDepthBudget += 1;
    }

    public String deserialize_str() throws DeserializationError {
        long len = deserialize_len();
        if (len < 0 || len > Integer.MAX_VALUE) {
            throw new DeserializationError("Incorrect length value for Java string");
        }
        byte[] content = new byte[(int) len];
        read(content);
        CharsetDecoder decoder = StandardCharsets.UTF_8.newDecoder();
        try {
            decoder.decode(ByteBuffer.wrap(content));
        } catch (CharacterCodingException ex) {
            throw new DeserializationError("Incorrect UTF8 string");
        }
        return new String(content);
    }

    public Bytes deserialize_bytes() throws DeserializationError {
        long len = deserialize_len();
        if (len < 0 || len > Integer.MAX_VALUE) {
            throw new DeserializationError("Incorrect length value for Java array");
        }
        byte[] content = new byte[(int) len];
        read(content);
        return new Bytes(content);
    }

    public Boolean deserialize_bool() throws DeserializationError {
        byte value = getByte();
        if (value == 0) {
            return Boolean.valueOf(false);
        }
        if (value == 1) {
            return Boolean.valueOf(true);
        }
        throw new DeserializationError("Incorrect boolean value");
    }

    public Unit deserialize_unit() throws DeserializationError {
        return new Unit();
    }

    public Character deserialize_char() throws DeserializationError {
        throw new DeserializationError("Not implemented: deserialize_char");
    }

    public @Unsigned Byte deserialize_u8() throws DeserializationError {
        return Byte.valueOf(getByte());
    }

    public @Unsigned Short deserialize_u16() throws DeserializationError {
        return Short.valueOf(getShort());
    }

    public @Unsigned Integer deserialize_u32() throws DeserializationError {
        return Integer.valueOf(getInt());
    }

    public @Unsigned Long deserialize_u64() throws DeserializationError {
        return Long.valueOf(getLong());
    }

    public @Unsigned @Int128 BigInteger deserialize_u128() throws DeserializationError {
        BigInteger signed = deserialize_i128();
        if (signed.compareTo(BigInteger.ZERO) >= 0) {
            return signed;
        } else {
            return signed.add(BigInteger.ONE.shiftLeft(128));
        }
    }

    public Byte deserialize_i8() throws DeserializationError {
        return Byte.valueOf(getByte());
    }

    public Short deserialize_i16() throws DeserializationError {
        return Short.valueOf(getShort());
    }

    public Integer deserialize_i32() throws DeserializationError {
        return Integer.valueOf(getInt());
    }

    public Long deserialize_i64() throws DeserializationError {
        return Long.valueOf(getLong());
    }

    public @Int128 BigInteger deserialize_i128() throws DeserializationError {
        byte[] content = new byte[16];
        read(content);
        byte[] reversed = new byte[16];
        for (int i = 0; i < 16; i++) {
            reversed[i] = content[15 - i];
        }
        return new BigInteger(reversed);
    }

    public boolean deserialize_option_tag() throws DeserializationError {
        return deserialize_bool().booleanValue();
    }

    public int get_buffer_offset() {
        return input.position();
    }

    static final String INPUT_NOT_LARGE_ENOUGH = "Input is not large enough";

    protected byte getByte()  throws DeserializationError {
        try {
            return input.get();
        } catch (java.nio.BufferUnderflowException e) {
            throw new DeserializationError(INPUT_NOT_LARGE_ENOUGH);
        }
    }

    protected short getShort()  throws DeserializationError {
        try {
            return input.getShort();
        } catch (java.nio.BufferUnderflowException e) {
            throw new DeserializationError(INPUT_NOT_LARGE_ENOUGH);
        }
    }

    protected int getInt()  throws DeserializationError {
        try {
            return input.getInt();
        } catch (java.nio.BufferUnderflowException e) {
            throw new DeserializationError(INPUT_NOT_LARGE_ENOUGH);
        }
    }

    protected long getLong()  throws DeserializationError {
        try {
            return input.getLong();
        } catch (java.nio.BufferUnderflowException e) {
            throw new DeserializationError(INPUT_NOT_LARGE_ENOUGH);
        }
    }

    protected float getFloat()  throws DeserializationError {
        try {
            return input.getFloat();
        } catch (java.nio.BufferUnderflowException e) {
            throw new DeserializationError(INPUT_NOT_LARGE_ENOUGH);
        }
    }

    protected double getDouble()  throws DeserializationError {
        try {
            return input.getDouble();
        } catch (java.nio.BufferUnderflowException e) {
            throw new DeserializationError(INPUT_NOT_LARGE_ENOUGH);
        }
    }

    protected void read(byte[] content)  throws DeserializationError {
        try {
            input.get(content);
        } catch (java.nio.BufferUnderflowException e) {
            throw new DeserializationError(INPUT_NOT_LARGE_ENOUGH);
        }
    }
}
