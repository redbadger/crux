import CoreData
import Foundation
import os.log

class KeyValueStore {
    private let logger = Logger(subsystem: "com.example.weather", category: "KeyValueStore")
    private let container: NSPersistentContainer
    private let context: NSManagedObjectContext
    private var isInitialized = false
    
    init() throws {
        logger.info("Initializing KeyValueStore")
        container = NSPersistentContainer(name: "KeyValueModel")
        
        let localLogger = logger
        var loadError: Error?
        
        // Use a semaphore to wait for Core Data initialization
        let semaphore = DispatchSemaphore(value: 0)
        
        container.loadPersistentStores { description, error in
            if let error = error {
                localLogger.error("Failed to load Core Data stack: \(error.localizedDescription)")
                loadError = error
            } else {
                localLogger.info("Core Data stack loaded successfully")
            }
            semaphore.signal()
        }
        
        semaphore.wait()
        
        if let error = loadError {
            throw error
        }
        
        context = container.viewContext
        context.automaticallyMergesChangesFromParent = true
        
        // Configure for better performance and safety
        context.mergePolicy = NSMergeByPropertyObjectTrumpMergePolicy
        
        isInitialized = true
        logger.info("KeyValueStore initialized successfully")
    }
    
    private func ensureInitialized() -> Bool {
        guard isInitialized else {
            logger.error("KeyValueStore not properly initialized")
            return false
        }
        return true
    }
    
    func get(key: String) -> String {
        guard ensureInitialized() else { return "" }
        
        let fetchRequest: NSFetchRequest<KeyValueEntity> = KeyValueEntity.fetchRequest()
        fetchRequest.predicate = NSPredicate(format: "key == %@", key)
        
        do {
            let results = try context.fetch(fetchRequest)
            let value = results.first?.value ?? ""
            logger.debug("Retrieved value for key '\(key)': \(value.isEmpty ? "empty" : "found")")
            return value
        } catch {
            logger.error("Failed to fetch value for key \(key): \(error.localizedDescription)")
            return ""
        }
    }
    
    func set(key: String, value: String) {
        guard ensureInitialized() else { return }
        
        let fetchRequest: NSFetchRequest<KeyValueEntity> = KeyValueEntity.fetchRequest()
        fetchRequest.predicate = NSPredicate(format: "key == %@", key)
        
        do {
            let results = try context.fetch(fetchRequest)
            if let existingEntity = results.first {
                existingEntity.value = value
                logger.debug("Updated existing value for key '\(key)'")
            } else {
                let newEntity = KeyValueEntity(context: context)
                newEntity.key = key
                newEntity.value = value
                logger.debug("Created new value for key '\(key)'")
            }
            
            if context.hasChanges {
                try context.save()
                logger.debug("Saved changes to Core Data")
            }
        } catch {
            logger.error("Failed to set value for key \(key): \(error.localizedDescription)")
            // Rollback changes on error
            context.rollback()
        }
    }
    
    func delete(key: String) {
        guard ensureInitialized() else { return }
        
        let fetchRequest: NSFetchRequest<KeyValueEntity> = KeyValueEntity.fetchRequest()
        fetchRequest.predicate = NSPredicate(format: "key == %@", key)
        
        do {
            let results = try context.fetch(fetchRequest)
            if let entity = results.first {
                context.delete(entity)
                if context.hasChanges {
                    try context.save()
                    logger.debug("Deleted key '\(key)' and saved changes")
                }
            } else {
                logger.debug("Key '\(key)' not found for deletion")
            }
        } catch {
            logger.error("Failed to delete key \(key): \(error.localizedDescription)")
            // Rollback changes on error
            context.rollback()
        }
    }
    
    func exists(key: String) -> Bool {
        guard ensureInitialized() else { return false }
        
        let fetchRequest: NSFetchRequest<KeyValueEntity> = KeyValueEntity.fetchRequest()
        fetchRequest.predicate = NSPredicate(format: "key == %@", key)
        
        do {
            let count = try context.count(for: fetchRequest)
            let exists = count > 0
            logger.debug("Key '\(key)' exists: \(exists)")
            return exists
        } catch {
            logger.error("Failed to check existence of key \(key): \(error.localizedDescription)")
            return false
        }
    }
    
    func listKeys(prefix: String, cursor: String?) -> [String] {
        guard ensureInitialized() else { return [] }
        
        let fetchRequest: NSFetchRequest<KeyValueEntity> = KeyValueEntity.fetchRequest()
        var predicates: [NSPredicate] = [NSPredicate(format: "key BEGINSWITH %@", prefix)]
        
        if let cursor = cursor, !cursor.isEmpty {
            predicates.append(NSPredicate(format: "key > %@", cursor))
        }
        
        fetchRequest.predicate = NSCompoundPredicate(andPredicateWithSubpredicates: predicates)
        fetchRequest.sortDescriptors = [NSSortDescriptor(key: "key", ascending: true)]
        
        do {
            let results = try context.fetch(fetchRequest)
            let keys = results.compactMap { $0.key }
            logger.debug("Listed \(keys.count) keys with prefix '\(prefix)'")
            return keys
        } catch {
            logger.error("Failed to list keys with prefix \(prefix): \(error.localizedDescription)")
            return []
        }
    }
} 
