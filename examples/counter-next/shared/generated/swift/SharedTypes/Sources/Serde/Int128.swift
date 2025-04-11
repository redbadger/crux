//  Copyright (c) Facebook, Inc. and its affiliates.

import Foundation

public struct Int128: Hashable {
    public var high: Int64
    public var low: UInt64

    public init(high: Int64, low: UInt64) {
        self.high = high
        self.low = low
    }
}
