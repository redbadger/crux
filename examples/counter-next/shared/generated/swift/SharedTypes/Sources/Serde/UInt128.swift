//  Copyright (c) Facebook, Inc. and its affiliates.

import Foundation

public struct UInt128: Hashable {
    public var high: UInt64
    public var low: UInt64

    public init(high: UInt64, low: UInt64) {
        self.high = high
        self.low = low
    }
}
