"use strict";
/**
 * Copyright (c) Facebook, Inc. and its affiliates
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.BincodeDeserializer = void 0;
const binaryDeserializer_1 = require("../serde/binaryDeserializer");
class BincodeDeserializer extends binaryDeserializer_1.BinaryDeserializer {
    deserializeLen() {
        return Number(this.deserializeU64());
    }
    deserializeVariantIndex() {
        return this.deserializeU32();
    }
    checkThatKeySlicesAreIncreasing(key1, key2) {
        return;
    }
}
exports.BincodeDeserializer = BincodeDeserializer;
