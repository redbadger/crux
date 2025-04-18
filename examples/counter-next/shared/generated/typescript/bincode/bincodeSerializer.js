"use strict";
/**
 * Copyright (c) Facebook, Inc. and its affiliates
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.BincodeSerializer = void 0;
const binarySerializer_1 = require("../serde/binarySerializer");
class BincodeSerializer extends binarySerializer_1.BinarySerializer {
    serializeLen(value) {
        this.serializeU64(value);
    }
    serializeVariantIndex(value) {
        this.serializeU32(value);
    }
    sortMapEntries(offsets) {
        return;
    }
}
exports.BincodeSerializer = BincodeSerializer;
