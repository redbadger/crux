"use strict";
/**
 * Copyright (c) Facebook, Inc. and its affiliates
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.BinarySerializer = void 0;
class BinarySerializer {
    constructor() {
        this.buffer = new ArrayBuffer(64);
        this.offset = 0;
    }
    ensureBufferWillHandleSize(bytes) {
        while (this.buffer.byteLength < this.offset + bytes) {
            const newBuffer = new ArrayBuffer(this.buffer.byteLength * 2);
            new Uint8Array(newBuffer).set(new Uint8Array(this.buffer));
            this.buffer = newBuffer;
        }
    }
    serialize(values) {
        this.ensureBufferWillHandleSize(values.length);
        new Uint8Array(this.buffer, this.offset).set(values);
        this.offset += values.length;
    }
    serializeStr(value) {
        this.serializeBytes(BinarySerializer.textEncoder.encode(value));
    }
    serializeBytes(value) {
        this.serializeLen(value.length);
        this.serialize(value);
    }
    serializeBool(value) {
        const byteValue = value ? 1 : 0;
        this.serialize(new Uint8Array([byteValue]));
    }
    // eslint-disable-next-line @typescript-eslint/no-unused-vars,@typescript-eslint/explicit-module-boundary-types
    serializeUnit(_value) {
        return;
    }
    serializeWithFunction(fn, bytesLength, value) {
        this.ensureBufferWillHandleSize(bytesLength);
        const dv = new DataView(this.buffer, this.offset);
        fn.apply(dv, [0, value, true]);
        this.offset += bytesLength;
    }
    serializeU8(value) {
        this.serialize(new Uint8Array([value]));
    }
    serializeU16(value) {
        this.serializeWithFunction(DataView.prototype.setUint16, 2, value);
    }
    serializeU32(value) {
        this.serializeWithFunction(DataView.prototype.setUint32, 4, value);
    }
    serializeU64(value) {
        const low = BigInt(value.toString()) & BinarySerializer.BIG_32Fs;
        const high = BigInt(value.toString()) >> BinarySerializer.BIG_32;
        // write little endian number
        this.serializeU32(Number(low));
        this.serializeU32(Number(high));
    }
    serializeU128(value) {
        const low = BigInt(value.toString()) & BinarySerializer.BIG_64Fs;
        const high = BigInt(value.toString()) >> BinarySerializer.BIG_64;
        // write little endian number
        this.serializeU64(low);
        this.serializeU64(high);
    }
    serializeI8(value) {
        const bytes = 1;
        this.ensureBufferWillHandleSize(bytes);
        new DataView(this.buffer, this.offset).setInt8(0, value);
        this.offset += bytes;
    }
    serializeI16(value) {
        const bytes = 2;
        this.ensureBufferWillHandleSize(bytes);
        new DataView(this.buffer, this.offset).setInt16(0, value, true);
        this.offset += bytes;
    }
    serializeI32(value) {
        const bytes = 4;
        this.ensureBufferWillHandleSize(bytes);
        new DataView(this.buffer, this.offset).setInt32(0, value, true);
        this.offset += bytes;
    }
    serializeI64(value) {
        const low = BigInt(value) & BinarySerializer.BIG_32Fs;
        const high = BigInt(value) >> BinarySerializer.BIG_32;
        // write little endian number
        this.serializeI32(Number(low));
        this.serializeI32(Number(high));
    }
    serializeI128(value) {
        const low = BigInt(value) & BinarySerializer.BIG_64Fs;
        const high = BigInt(value) >> BinarySerializer.BIG_64;
        // write little endian number
        this.serializeI64(low);
        this.serializeI64(high);
    }
    serializeOptionTag(value) {
        this.serializeBool(value);
    }
    getBufferOffset() {
        return this.offset;
    }
    getBytes() {
        return new Uint8Array(this.buffer).slice(0, this.offset);
    }
    serializeChar(_value) {
        throw new Error("Method serializeChar not implemented.");
    }
    serializeF32(value) {
        const bytes = 4;
        this.ensureBufferWillHandleSize(bytes);
        new DataView(this.buffer, this.offset).setFloat32(0, value, true);
        this.offset += bytes;
    }
    serializeF64(value) {
        const bytes = 8;
        this.ensureBufferWillHandleSize(bytes);
        new DataView(this.buffer, this.offset).setFloat64(0, value, true);
        this.offset += bytes;
    }
}
exports.BinarySerializer = BinarySerializer;
BinarySerializer.BIG_32 = BigInt(32);
BinarySerializer.BIG_64 = BigInt(64);
BinarySerializer.BIG_32Fs = BigInt("4294967295");
BinarySerializer.BIG_64Fs = BigInt("18446744073709551615");
BinarySerializer.textEncoder = new TextEncoder();
