/**
 * Copyright (c) Facebook, Inc. and its affiliates
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */
import { BinarySerializer } from "../serde/binarySerializer";
export declare class BincodeSerializer extends BinarySerializer {
    serializeLen(value: number): void;
    serializeVariantIndex(value: number): void;
    sortMapEntries(offsets: number[]): void;
}
