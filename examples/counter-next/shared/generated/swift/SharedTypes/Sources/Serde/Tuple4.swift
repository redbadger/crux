//  Copyright (c) Facebook, Inc. and its affiliates.

import Foundation

// Swift tuples are not properly equatable or hashable. This ruins our data model so we must use homemade structs as in Java.

public struct Tuple4<T0: Hashable, T1: Hashable, T2: Hashable, T3: Hashable>: Hashable {
    public var field0: T0
    public var field1: T1
    public var field2: T2
    public var field3: T3

    public init(_ field0: T0, _ field1: T1, _ field2: T2, _ field3: T3) {
        self.field0 = field0
        self.field1 = field1
        self.field2 = field2
        self.field3 = field3
    }
}
