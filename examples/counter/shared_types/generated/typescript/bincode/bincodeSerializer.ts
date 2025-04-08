/**
 * Copyright (c) Facebook, Inc. and its affiliates
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */

import { BinarySerializer } from "../serde/binarySerializer";

export class BincodeSerializer extends BinarySerializer {
  serializeLen(value: number): void {
    this.serializeU64(value);
  }

  public serializeVariantIndex(value: number): void {
    this.serializeU32(value);
  }

  public sortMapEntries(offsets: number[]): void {
    return;
  }
}
