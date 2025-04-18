//  Copyright (c) Facebook, Inc. and its affiliates.

import Foundation

public struct Slice: Equatable {
    public var start: Int
    public var end: Int

    public init(start: Int, end: Int) {
        self.start = start
        self.end = end
    }
}
