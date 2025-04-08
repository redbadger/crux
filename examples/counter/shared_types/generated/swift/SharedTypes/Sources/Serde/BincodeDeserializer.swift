//  Copyright (c) Facebook, Inc. and its affiliates.

import Foundation

public class BincodeDeserializer: BinaryDeserializer {
    public let MAX_LENGTH: Int = 1 << 31 - 1

    public init(input: [UInt8]) {
        super.init(input: input, maxContainerDepth: Int.max)
    }

    override public func deserialize_len() throws -> Int {
        let value = try deserialize_i64()
        if value < 0 || value > MAX_LENGTH {
            throw DeserializationError.invalidInput(issue: "Incorrect length value")
        }
        return Int(value)
    }

    override public func deserialize_f32() throws -> Float {
        let num = try deserialize_u32()
        return Float(bitPattern: num)
    }

    override public func deserialize_f64() throws -> Double {
        let num = try deserialize_u64()
        return Double(bitPattern: num)
    }

    override public func deserialize_variant_index() throws -> UInt32 {
        return try deserialize_u32()
    }

    override public func check_that_key_slices_are_increasing(key1 _: Slice, key2 _: Slice) throws {
        // Nothing to do
    }
}
