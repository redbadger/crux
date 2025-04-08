//  Copyright (c) Facebook, Inc. and its affiliates.

// See https://forums.swift.org/t/using-indirect-modifier-for-struct-properties/37600/16
@propertyWrapper
public enum Indirect<T>: Hashable where T: Hashable {
    indirect case wrapped(T)

    public init(wrappedValue initialValue: T) {
        self = .wrapped(initialValue)
    }

    public var wrappedValue: T {
        get { switch self { case let .wrapped(x): return x } }
        set { self = .wrapped(newValue) }
    }
}
