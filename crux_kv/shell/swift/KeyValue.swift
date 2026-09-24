import Foundation

/// The shell side of `crux_kv`, as one method per operation.
///
/// Each method's shape is the generated `EffectHandler`'s, so an app's handler
/// holds an instance and delegates in one line:
///
/// ```swift
/// struct MyHandler: EffectHandler {
///     let kv = UserDefaultsKeyValueHandler()
///
///     func kvGet(_ operation: GetValue) async -> ValueResult { await kv.get(operation) }
///     func kvSet(_ operation: SetValue) async -> ValueResult { await kv.set(operation) }
/// }
/// ```
///
/// Conform your own type to it to put the store somewhere else — the keychain,
/// a database, a file — or to replace a single method and leave the rest.
///
/// ## What each operation answers with
///
/// `get` answers with what is stored under the key, `Value.none` when nothing
/// is. `set` and `delete` answer with the value they replaced or removed, which
/// is also `Value.none` when there was none. Nothing here is an error unless
/// the store itself failed; a missing key is an answer.
public protocol KeyValueHandler: Sendable {
    /// Read the bytes stored under `operation.key`.
    func get(_ operation: GetValue) async -> ValueResult
    /// Write `operation.value`, answering with the value it replaced.
    func set(_ operation: SetValue) async -> ValueResult
    /// Remove `operation.key`, answering with the value it removed.
    func delete(_ operation: DeleteValue) async -> ValueResult
    /// Whether `operation.key` is in the store.
    func exists(_ operation: KeyExists) async -> ExistsResult
    /// The keys starting with `operation.prefix`, from `operation.cursor`.
    func listKeys(_ operation: ListKeys) async -> KeysResult
}

/// A `KeyValueHandler` over `UserDefaults`, storing each value as `Data`.
///
/// ## Which defaults, and which domain
///
/// A read goes through the whole search list, as every `UserDefaults` read
/// does, so a key the app already uses for a preference is the same key here.
/// `listKeys`, though, reads back only the one persistent domain this store
/// writes to — `domain` names it — because the search list also holds the
/// global domain and the system's own keys, and none of those is a key the
/// app put here.
///
/// The domain defaults to the main bundle's identifier, which is the domain
/// the standard defaults write to. Pass a suite instead, with
/// `init(suiteName:)`, to keep the store's keys apart from the app's
/// preferences; that initialiser names the domain for you. A `UserDefaults`
/// built by hand and handed to `init(defaults:domain:)` needs its domain
/// named too, or `listKeys` will read a domain it does not write to.
///
/// ## Size
///
/// `UserDefaults` is a preferences store, not a database: it is read into
/// memory whole and written out whole. Documents and caches belong in a file.
/// `@unchecked`, because `UserDefaults` is thread-safe but predates `Sendable`
/// and is not marked as such.
public final class UserDefaultsKeyValueHandler: KeyValueHandler, @unchecked Sendable {
    private let defaults: UserDefaults
    private let domain: String?

    /// Stores everything in `defaults`, the standard ones unless told
    /// otherwise, writing to the persistent domain `domain` names.
    ///
    /// - Parameter domain: the domain `defaults` writes to, and the one
    ///   `listKeys` reads back. The main bundle's identifier is the standard
    ///   defaults' own domain, and is the default here; a process without a
    ///   bundle identifier has none, and `listKeys` then answers with an
    ///   error rather than with the whole search list.
    public init(defaults: UserDefaults = .standard, domain: String? = Bundle.main.bundleIdentifier) {
        self.defaults = defaults
        self.domain = domain
    }

    /// Stores everything in the suite `suiteName` names, creating it if it does
    /// not exist — the standard defaults if it cannot be opened.
    public convenience init(suiteName: String) {
        self.init(defaults: UserDefaults(suiteName: suiteName) ?? .standard, domain: suiteName)
    }

    public func get(_ operation: GetValue) async -> ValueResult {
        .ok(Self.value(defaults.data(forKey: operation.key)))
    }

    public func set(_ operation: SetValue) async -> ValueResult {
        let previous = defaults.data(forKey: operation.key)
        defaults.set(Data(operation.value), forKey: operation.key)
        return .ok(Self.value(previous))
    }

    public func delete(_ operation: DeleteValue) async -> ValueResult {
        let previous = defaults.data(forKey: operation.key)
        defaults.removeObject(forKey: operation.key)
        return .ok(Self.value(previous))
    }

    /// `data(forKey:)`, not `object(forKey:)`, so that this agrees with `get`:
    /// a key whose value is not `Data` — one of the system's, say — is not a
    /// key this store holds, and `get` already answers `.none` for it.
    public func exists(_ operation: KeyExists) async -> ExistsResult {
        .ok(defaults.data(forKey: operation.key) != nil)
    }

    /// One page holds every remaining key, so the answer's cursor is always 0:
    /// the domain is small enough to sort in one go, and paging it would only
    /// invite a caller to hold a cursor across a write.
    ///
    /// The keys come from the store's own persistent domain, not from
    /// `dictionaryRepresentation()`, which is the union of the search list and
    /// would answer with the global domain's keys — `AppleLanguages` and the
    /// rest — none of which the app ever wrote here.
    public func listKeys(_ operation: ListKeys) async -> KeysResult {
        guard let domain else {
            return .err(
                .other(
                    message: """
                        listKeys needs the name of the domain the store writes to, \
                        and this process has no bundle identifier to fall back on: \
                        pass `domain:`, or use `init(suiteName:)`.
                        """
                )
            )
        }

        let keys = (defaults.persistentDomain(forName: domain) ?? [:]).keys
            .filter { $0.hasPrefix(operation.prefix) }
            .sorted()

        guard operation.cursor <= UInt64(keys.count) else {
            return .err(.cursorNotFound)
        }

        return .ok(KeyPage(keys: Array(keys[Int(operation.cursor)...]), nextCursor: 0))
    }

    private static func value(_ data: Data?) -> Value {
        guard let data else { return .none }
        return .bytes([UInt8](data))
    }
}
