import App
import Foundation
import Security

private let logger = Log.secret

#if os(iOS)
    private let keychainService = "com.crux.examples.weather.ios"
#else
    private let keychainService = "com.crux.examples.weather"
#endif

extension Core {
    func resolveSecret(request: SecretRequest, requestId: UInt32) {
        let response: SecretResponse
        switch request {
        case let .fetch(key):
            logger.debug("fetching secret: \(key)")
            if let value = keychainGet(key: key) {
                logger.debug("secret fetched: \(key)")
                response = .fetched(key, value)
            } else {
                logger.debug("secret not found: \(key)")
                response = .missing(key)
            }
        case let .store(key, value):
            logger.debug("storing secret: \(key)")
            do {
                try keychainSave(key: key, value: value)
                logger.debug("secret stored: \(key)")
                response = .stored(key)
            } catch {
                logger.warning("store failed for \(key): \(error)")
                response = .storeError(error.localizedDescription)
            }
        case let .delete(key):
            logger.debug("deleting secret: \(key)")
            do {
                try keychainDelete(key: key)
                logger.debug("secret deleted: \(key)")
                response = .deleted(key)
            } catch {
                logger.warning("delete failed for \(key): \(error)")
                response = .deleteError(error.localizedDescription)
            }
        }
        resolve(requestId: requestId, serialize: { try response.bincodeSerialize() })
    }
}

// MARK: - Keychain Operations

private func keychainSave(key: String, value: String) throws {
    guard let data = value.data(using: .utf8) else {
        throw KeychainError.encodingFailed
    }

    // NOTE: On macOS we omit kSecUseDataProtectionKeychain because it
    // requires code signing with a development team. A production app
    // should use the Data Protection Keychain on all platforms for
    // stronger security. See Apple's documentation for details.
    var query: [CFString: Any] = [
        kSecClass: kSecClassGenericPassword,
        kSecAttrAccount: key,
        kSecAttrService: keychainService,
        kSecValueData: data
    ]
    #if os(iOS)
        query[kSecUseDataProtectionKeychain] = true as CFBoolean
    #endif

    let status = SecItemAdd(query as CFDictionary, nil)

    if status == errSecDuplicateItem {
        var updateQuery: [CFString: Any] = [
            kSecClass: kSecClassGenericPassword,
            kSecAttrAccount: key,
            kSecAttrService: keychainService
        ]
        #if os(iOS)
            updateQuery[kSecUseDataProtectionKeychain] = true as CFBoolean
        #endif
        let updateStatus = SecItemUpdate(
            updateQuery as CFDictionary,
            [kSecValueData: data] as CFDictionary
        )
        guard updateStatus == errSecSuccess else {
            throw KeychainError.unhandledError(status: updateStatus)
        }
    } else if status != errSecSuccess {
        throw KeychainError.unhandledError(status: status)
    }
}

private func keychainGet(key: String) -> String? {
    var query: [CFString: Any] = [
        kSecClass: kSecClassGenericPassword,
        kSecAttrAccount: key,
        kSecAttrService: keychainService,
        kSecReturnData: true,
        kSecMatchLimit: kSecMatchLimitOne
    ]
    #if os(iOS)
        query[kSecUseDataProtectionKeychain] = true as CFBoolean
    #endif

    var result: AnyObject?
    let status = SecItemCopyMatching(query as CFDictionary, &result)

    guard status == errSecSuccess,
          let data = result as? Data,
          let value = String(data: data, encoding: .utf8)
    else {
        if status != errSecItemNotFound {
            logger.warning("keychain lookup failed with status: \(status)")
        }
        return nil
    }

    return value
}

private func keychainDelete(key: String) throws {
    var query: [CFString: Any] = [
        kSecClass: kSecClassGenericPassword,
        kSecAttrAccount: key,
        kSecAttrService: keychainService
    ]
    #if os(iOS)
        query[kSecUseDataProtectionKeychain] = true as CFBoolean
    #endif

    let status = SecItemDelete(query as CFDictionary)
    guard status == errSecSuccess || status == errSecItemNotFound else {
        throw KeychainError.unhandledError(status: status)
    }
}

private enum KeychainError: Error, LocalizedError {
    case encodingFailed
    case unhandledError(status: OSStatus)

    var errorDescription: String? {
        switch self {
        case .encodingFailed: "Failed to encode secret as UTF-8"
        case let .unhandledError(status): "Keychain error: \(status)"
        }
    }
}
