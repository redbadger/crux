/**
 * Copyright (c) Facebook, Inc. and its affiliates
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */
import { Serializer } from "./serializer";
export declare abstract class BinarySerializer implements Serializer {
    private static readonly BIG_32;
    private static readonly BIG_64;
    private static readonly BIG_32Fs;
    private static readonly BIG_64Fs;
    private static readonly textEncoder;
    private buffer;
    private offset;
    constructor();
    private ensureBufferWillHandleSize;
    protected serialize(values: Uint8Array): void;
    abstract serializeLen(value: number): void;
    abstract serializeVariantIndex(value: number): void;
    abstract sortMapEntries(offsets: number[]): void;
    serializeStr(value: string): void;
    serializeBytes(value: Uint8Array): void;
    serializeBool(value: boolean): void;
    serializeUnit(_value: null): void;
    private serializeWithFunction;
    serializeU8(value: number): void;
    serializeU16(value: number): void;
    serializeU32(value: number): void;
    serializeU64(value: BigInt | number): void;
    serializeU128(value: BigInt | number): void;
    serializeI8(value: number): void;
    serializeI16(value: number): void;
    serializeI32(value: number): void;
    serializeI64(value: bigint | number): void;
    serializeI128(value: bigint | number): void;
    serializeOptionTag(value: boolean): void;
    getBufferOffset(): number;
    getBytes(): Uint8Array;
    serializeChar(_value: string): void;
    serializeF32(value: number): void;
    serializeF64(value: number): void;
}
