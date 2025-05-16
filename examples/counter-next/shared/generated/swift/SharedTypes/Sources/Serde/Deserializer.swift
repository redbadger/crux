//  Copyright (c) Facebook, Inc. and its affiliates.

import Foundation

public enum DeserializationError: Error {
    case invalidInput(issue: String)
}

public protocol Deserializer {
    func deserialize_str() throws -> String
    func deserialize_bytes() throws -> [UInt8]
    func deserialize_bool() throws -> Bool
    func deserialize_unit() throws -> Unit
    func deserialize_char() throws -> Character
    func deserialize_f32() throws -> Float
    func deserialize_f64() throws -> Double
    func deserialize_u8() throws -> UInt8
    func deserialize_u16() throws -> UInt16
    func deserialize_u32() throws -> UInt32
    func deserialize_u64() throws -> UInt64
    func deserialize_u128() throws -> UInt128
    func deserialize_i8() throws -> Int8
    func deserialize_i16() throws -> Int16
    func deserialize_i32() throws -> Int32
    func deserialize_i64() throws -> Int64
    func deserialize_i128() throws -> Int128
    func deserialize_len() throws -> Int
    func deserialize_variant_index() throws -> UInt32
    func deserialize_option_tag() throws -> Bool
    func get_buffer_offset() -> Int
    func check_that_key_slices_are_increasing(key1: Slice, key2: Slice) throws
    func increase_container_depth() throws
    func decrease_container_depth() throws
}
