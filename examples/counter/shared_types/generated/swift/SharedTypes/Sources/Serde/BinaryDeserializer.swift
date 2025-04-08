//  Copyright (c) Facebook, Inc. and its affiliates.

import Foundation

public class BinaryDeserializer: Deserializer {
    let input: [UInt8]
    private var location: Int
    private var containerDepthBudget: Int

    init(input: [UInt8], maxContainerDepth: Int) {
        self.input = input
        location = 0
        containerDepthBudget = maxContainerDepth
    }

    private func readBytes(count: Int) throws -> [UInt8] {
        let newLocation = location + count
        if newLocation > input.count {
            throw DeserializationError.invalidInput(issue: "Input is too small")
        }
        let bytes = input[location ..< newLocation]
        location = newLocation
        return Array(bytes)
    }

    public func deserialize_len() throws -> Int {
        assertionFailure("Not implemented")
        return 0
    }

    public func deserialize_variant_index() throws -> UInt32 {
        assertionFailure("Not implemented")
        return 0
    }

    public func deserialize_char() throws -> Character {
        throw DeserializationError.invalidInput(issue: "Not implemented: char deserialization")
    }

    public func deserialize_f32() throws -> Float {
        throw DeserializationError.invalidInput(issue: "Not implemented: f32 deserialization")
    }

    public func deserialize_f64() throws -> Double {
        throw DeserializationError.invalidInput(issue: "Not implemented: f64 deserialization")
    }

    public func increase_container_depth() throws {
        if containerDepthBudget == 0 {
            throw DeserializationError.invalidInput(issue: "Exceeded maximum container depth")
        }
        containerDepthBudget -= 1
    }

    public func decrease_container_depth() {
        containerDepthBudget += 1
    }

    public func deserialize_str() throws -> String {
        let bytes = try deserialize_bytes()
        guard let value = String(bytes: bytes, encoding: .utf8) else {
            throw DeserializationError.invalidInput(issue: "Incorrect UTF8 string")
        }
        return value
    }

    public func deserialize_bytes() throws -> [UInt8] {
        let len = try deserialize_len()
        let content = try readBytes(count: len)
        return content
    }

    public func deserialize_bool() throws -> Bool {
        let value = try deserialize_u8()
        switch value {
        case 0: return false
        case 1: return true
        default: throw DeserializationError.invalidInput(issue: "Incorrect value for boolean: \(value)")
        }
    }

    public func deserialize_unit() throws -> Unit {
        return Unit()
    }

    public func deserialize_u8() throws -> UInt8 {
        let bytes = try readBytes(count: 1)
        return bytes[0]
    }

    public func deserialize_u16() throws -> UInt16 {
        let bytes = try readBytes(count: 2)
        var x = UInt16(bytes[0])
        x += UInt16(bytes[1]) << 8
        return x
    }

    public func deserialize_u32() throws -> UInt32 {
        let bytes = try readBytes(count: 4)
        var x = UInt32(bytes[0])
        x += UInt32(bytes[1]) << 8
        x += UInt32(bytes[2]) << 16
        x += UInt32(bytes[3]) << 24
        return x
    }

    public func deserialize_u64() throws -> UInt64 {
        let bytes = try readBytes(count: 8)
        var x = UInt64(bytes[0])
        x += UInt64(bytes[1]) << 8
        x += UInt64(bytes[2]) << 16
        x += UInt64(bytes[3]) << 24
        x += UInt64(bytes[4]) << 32
        x += UInt64(bytes[5]) << 40
        x += UInt64(bytes[6]) << 48
        x += UInt64(bytes[7]) << 56
        return x
    }

    public func deserialize_u128() throws -> UInt128 {
        let low = try deserialize_u64()
        let high = try deserialize_u64()
        return UInt128(high: high, low: low)
    }

    public func deserialize_i8() throws -> Int8 {
        return Int8(bitPattern: try deserialize_u8())
    }

    public func deserialize_i16() throws -> Int16 {
        return Int16(bitPattern: try deserialize_u16())
    }

    public func deserialize_i32() throws -> Int32 {
        return Int32(bitPattern: try deserialize_u32())
    }

    public func deserialize_i64() throws -> Int64 {
        return Int64(bitPattern: try deserialize_u64())
    }

    public func deserialize_i128() throws -> Int128 {
        let low = try deserialize_u64()
        let high = try deserialize_i64()
        return Int128(high: high, low: low)
    }

    public func deserialize_option_tag() throws -> Bool {
        let value = try deserialize_u8()
        switch value {
        case 0: return false
        case 1: return true
        default: throw DeserializationError.invalidInput(issue: "Incorrect value for option tag: \(value)")
        }
    }

    public func get_buffer_offset() -> Int {
        return location
    }

    public func check_that_key_slices_are_increasing(key1 _: Slice, key2 _: Slice) throws {
        assertionFailure("Not implemented")
    }
}
