//  Copyright (c) Facebook, Inc. and its affiliates.

import Foundation

public enum SerializationError: Error {
    case invalidValue(issue: String)
}

public protocol Serializer {
    func serialize_str(value: String) throws
    func serialize_bytes(value: [UInt8]) throws
    func serialize_bool(value: Bool) throws
    func serialize_unit(value: Unit) throws
    func serialize_char(value: Character) throws
    func serialize_f32(value: Float) throws
    func serialize_f64(value: Double) throws
    func serialize_u8(value: UInt8) throws
    func serialize_u16(value: UInt16) throws
    func serialize_u32(value: UInt32) throws
    func serialize_u64(value: UInt64) throws
    func serialize_u128(value: UInt128) throws
    func serialize_i8(value: Int8) throws
    func serialize_i16(value: Int16) throws
    func serialize_i32(value: Int32) throws
    func serialize_i64(value: Int64) throws
    func serialize_i128(value: Int128) throws
    func serialize_len(value: Int) throws
    func serialize_variant_index(value: UInt32) throws
    func serialize_option_tag(value: Bool) throws
    func increase_container_depth() throws
    func decrease_container_depth() throws
    func get_buffer_offset() -> Int
    func sort_map_entries(offsets: [Int])
    func get_bytes() -> [UInt8]
}
