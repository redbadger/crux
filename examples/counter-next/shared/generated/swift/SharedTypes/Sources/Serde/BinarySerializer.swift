//  Copyright (c) Facebook, Inc. and its affiliates.

import Foundation

public class BinarySerializer: Serializer {
    var output: [UInt8]
    private var containerDepthBudget: Int

    public init(maxContainerDepth: Int) {
        output = []
        output.reserveCapacity(64)
        containerDepthBudget = maxContainerDepth
    }

    public func increase_container_depth() throws {
        if containerDepthBudget == 0 {
            throw SerializationError.invalidValue(issue: "Exceeded maximum container depth")
        }
        containerDepthBudget -= 1
    }

    public func decrease_container_depth() {
        containerDepthBudget += 1
    }

    public func serialize_char(value _: Character) throws {
        throw SerializationError.invalidValue(issue: "Not implemented: char serialization")
    }

    public func serialize_f32(value: Float) throws {
        throw SerializationError.invalidValue(issue: "Not implemented: f32 serialization")
    }

    public func serialize_f64(value: Double) throws {
        throw SerializationError.invalidValue(issue: "Not implemented: f64 serialization")
    }

    public func get_bytes() -> [UInt8] {
        return output
    }

    public func serialize_str(value: String) throws {
        try serialize_bytes(value: Array(value.utf8))
    }

    public func serialize_bytes(value: [UInt8]) throws {
        try serialize_len(value: value.count)
        output.append(contentsOf: value)
    }

    public func serialize_bool(value: Bool) throws {
        writeByte(value ? 1 : 0)
    }

    public func serialize_unit(value _: Unit) throws {}

    func writeByte(_ value: UInt8) {
        output.append(value)
    }

    public func serialize_u8(value: UInt8) throws {
        writeByte(value)
    }

    public func serialize_u16(value: UInt16) throws {
        writeByte(UInt8(truncatingIfNeeded: value))
        writeByte(UInt8(truncatingIfNeeded: value >> 8))
    }

    public func serialize_u32(value: UInt32) throws {
        writeByte(UInt8(truncatingIfNeeded: value))
        writeByte(UInt8(truncatingIfNeeded: value >> 8))
        writeByte(UInt8(truncatingIfNeeded: value >> 16))
        writeByte(UInt8(truncatingIfNeeded: value >> 24))
    }

    public func serialize_u64(value: UInt64) throws {
        writeByte(UInt8(truncatingIfNeeded: value))
        writeByte(UInt8(truncatingIfNeeded: value >> 8))
        writeByte(UInt8(truncatingIfNeeded: value >> 16))
        writeByte(UInt8(truncatingIfNeeded: value >> 24))
        writeByte(UInt8(truncatingIfNeeded: value >> 32))
        writeByte(UInt8(truncatingIfNeeded: value >> 40))
        writeByte(UInt8(truncatingIfNeeded: value >> 48))
        writeByte(UInt8(truncatingIfNeeded: value >> 56))
    }

    public func serialize_u128(value: UInt128) throws {
        try serialize_u64(value: value.low)
        try serialize_u64(value: value.high)
    }

    public func serialize_i8(value: Int8) throws {
        try serialize_u8(value: UInt8(bitPattern: value))
    }

    public func serialize_i16(value: Int16) throws {
        try serialize_u16(value: UInt16(bitPattern: value))
    }

    public func serialize_i32(value: Int32) throws {
        try serialize_u32(value: UInt32(bitPattern: value))
    }

    public func serialize_i64(value: Int64) throws {
        try serialize_u64(value: UInt64(bitPattern: value))
    }

    public func serialize_i128(value: Int128) throws {
        try serialize_u64(value: value.low)
        try serialize_i64(value: value.high)
    }

    public func serialize_option_tag(value: Bool) throws {
        writeByte(value ? 1 : 0)
    }

    public func get_buffer_offset() -> Int {
        return output.count
    }

    public func serialize_len(value _: Int) throws {
        assertionFailure("Not implemented")
    }

    public func serialize_variant_index(value _: UInt32) throws {
        assertionFailure("Not implemented")
    }

    public func sort_map_entries(offsets _: [Int]) {
        assertionFailure("Not implemented")
    }
}
