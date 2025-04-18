/**
 * Copyright (c) Facebook, Inc. and its affiliates
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */
import { Deserializer } from "./deserializer";
export declare abstract class BinaryDeserializer implements Deserializer {
    private static readonly BIG_32;
    private static readonly BIG_64;
    private static readonly textDecoder;
    buffer: ArrayBuffer;
    offset: number;
    constructor(data: Uint8Array);
    private read;
    abstract deserializeLen(): number;
    abstract deserializeVariantIndex(): number;
    abstract checkThatKeySlicesAreIncreasing(key1: [number, number], key2: [number, number]): void;
    deserializeStr(): string;
    deserializeBytes(): Uint8Array;
    deserializeBool(): boolean;
    deserializeUnit(): null;
    deserializeU8(): number;
    deserializeU16(): number;
    deserializeU32(): number;
    deserializeU64(): bigint;
    deserializeU128(): bigint;
    deserializeI8(): number;
    deserializeI16(): number;
    deserializeI32(): number;
    deserializeI64(): bigint;
    deserializeI128(): bigint;
    deserializeOptionTag(): boolean;
    getBufferOffset(): number;
    deserializeChar(): string;
    deserializeF32(): number;
    deserializeF64(): number;
}
