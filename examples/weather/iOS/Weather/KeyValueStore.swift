import CoreData
import Foundation
import os.log

class KeyValueStore {
    private let logger = Logger(subsystem: "com.example.weather", category: "KeyValueStore")
    private let container: NSPersistentContainer
    private let context: NSManagedObjectContext
    
    init() {
        container = NSPersistentContainer(name: "KeyValueModel")
        
        let localLogger = logger
        
        container.loadPersistentStores { description, error in
            if let error = error {
                localLogger.error("Failed to load Core Data stack: \(error.localizedDescription)")
                fatalError("Failed to load Core Data stack: \(error)")
            }
        }
        
        context = container.viewContext
    }
    
    func get(key: String) -> String {
        let fetchRequest: NSFetchRequest<KeyValueEntity> = KeyValueEntity.fetchRequest()
        fetchRequest.predicate = NSPredicate(format: "key == %@", key)
        
        do {
            let results = try context.fetch(fetchRequest)
            return results.first?.value ?? ""
        } catch {
            logger.error("Failed to fetch value for key \(key): \(error.localizedDescription)")
            return ""
        }
    }
    
    func set(key: String, value: String) {
        let fetchRequest: NSFetchRequest<KeyValueEntity> = KeyValueEntity.fetchRequest()
        fetchRequest.predicate = NSPredicate(format: "key == %@", key)
        
        do {
            let results = try context.fetch(fetchRequest)
            if let existingEntity = results.first {
                existingEntity.value = value
            } else {
                let newEntity = KeyValueEntity(context: context)
                newEntity.key = key
                newEntity.value = value
            }
            try context.save()
        } catch {
            logger.error("Failed to set value for key \(key): \(error.localizedDescription)")
        }
    }
    
    func delete(key: String) {
        let fetchRequest: NSFetchRequest<KeyValueEntity> = KeyValueEntity.fetchRequest()
        fetchRequest.predicate = NSPredicate(format: "key == %@", key)
        
        do {
            let results = try context.fetch(fetchRequest)
            if let entity = results.first {
                context.delete(entity)
                try context.save()
            }
        } catch {
            logger.error("Failed to delete key \(key): \(error.localizedDescription)")
        }
    }
    
    func exists(key: String) -> Bool {
        let fetchRequest: NSFetchRequest<KeyValueEntity> = KeyValueEntity.fetchRequest()
        fetchRequest.predicate = NSPredicate(format: "key == %@", key)
        
        do {
            let count = try context.count(for: fetchRequest)
            return count > 0
        } catch {
            logger.error("Failed to check existence of key \(key): \(error.localizedDescription)")
            return false
        }
    }
    
    func listKeys(prefix: String, cursor: String?) -> [String] {
        let fetchRequest: NSFetchRequest<KeyValueEntity> = KeyValueEntity.fetchRequest()
        var predicates: [NSPredicate] = [NSPredicate(format: "key BEGINSWITH %@", prefix)]
        
        if let cursor = cursor, !cursor.isEmpty {
            predicates.append(NSPredicate(format: "key > %@", cursor))
        }
        
        fetchRequest.predicate = NSCompoundPredicate(andPredicateWithSubpredicates: predicates)
        fetchRequest.sortDescriptors = [NSSortDescriptor(key: "key", ascending: true)]
        
        do {
            let results = try context.fetch(fetchRequest)
            return results.compactMap { $0.key }
        } catch {
            logger.error("Failed to list keys with prefix \(prefix): \(error.localizedDescription)")
            return []
        }
    }
} 
