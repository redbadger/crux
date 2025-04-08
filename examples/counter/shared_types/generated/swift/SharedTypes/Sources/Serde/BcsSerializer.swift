//  Copyright (c) Facebook, Inc. and its affiliates.

import Foundation

public class BcsSerializer: BinarySerializer {
    public let MAX_LENGTH: Int = 1 << 31 - 1
    public let MAX_CONTAINER_DEPTH: Int = 500

    public init() {
        super.init(maxContainerDepth: MAX_CONTAINER_DEPTH)
    }

    private func serialize_u32_as_uleb128(value: UInt32) throws {
        var input = value
        while input >= 0x80 {
            writeByte(UInt8((value & 0x7F) | 0x80))
            input >>= 7
        }
        writeByte(UInt8(input))
    }

    override public func serialize_len(value: Int) throws {
        if value < 0 || value > MAX_LENGTH {
            throw SerializationError.invalidValue(issue: "Invalid length value")
        }
        try serialize_u32_as_uleb128(value: UInt32(value))
    }

    override public func serialize_variant_index(value: UInt32) throws {
        try serialize_u32_as_uleb128(value: value)
    }

    override public func sort_map_entries(offsets: [Int]) {
        if offsets.count <= 1 {
            return
        }
        let offset0 = offsets[0]
        var slices: [Slice] = []
        slices.reserveCapacity(offsets.count)
        for i in 0 ..< (offsets.count - 1) {
            slices.append(Slice(start: offsets[i], end: offsets[i + 1]))
        }
        slices.append(Slice(start: offsets[offsets.count - 1], end: output.count))
        slices.sort(by: { key1, key2 in
            output[key1.start ..< key1.end].lexicographicallyPrecedes(output[key2.start ..< key2.end])
        })

        let content = output
        var position = offset0
        for slice in slices {
            for i in slice.start ..< slice.end {
                output[position] = content[i]
                position += 1
            }
        }
    }
}
