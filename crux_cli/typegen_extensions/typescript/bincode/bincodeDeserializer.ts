/**
 * Copyright (c) Facebook, Inc. and its affiliates
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */

import { BinaryDeserializer } from "../serde/binaryDeserializer";

export class BincodeDeserializer extends BinaryDeserializer {
  deserializeLen(): number {
    return Number(this.deserializeU64());
  }

  public deserializeVariantIndex(): number {
    return this.deserializeU32();
  }

  checkThatKeySlicesAreIncreasing(
    key1: [number, number],
    key2: [number, number],
  ): void {
    return;
  }
}
