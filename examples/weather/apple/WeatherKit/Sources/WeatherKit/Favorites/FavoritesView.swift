import App
import SwiftUI

struct FavoritesView: View {
    @Environment(CoreUpdater.self) var update
    let model: FavoritesViewModel

    private var isConfirmingDelete: Bool {
        if case .confirmDelete = model.workflow { return true }
        return false
    }

    var body: some View {
        Group {
            if case let .add(addModel) = model.workflow {
                AddFavoriteView(model: addModel)
                    .navigationTitle("Add Favorite")
                    .toolbar {
                        ToolbarItem(placement: .cancellationAction) {
                            Button("Cancel") {
                                update(.active(.favorites(.workflow(.add(.cancel)))))
                            }
                        }
                    }
            } else {
                favoritesList
                    .navigationTitle("Favorites")
                    .toolbar {
                        ToolbarItem(placement: .cancellationAction) {
                            Button("Done") {
                                update(.active(.favorites(.goToHome)))
                            }
                        }
                        ToolbarItem(placement: .primaryAction) {
                            Button {
                                update(.active(.favorites(.requestAddFavorite)))
                            } label: {
                                Image(systemName: "plus")
                            }
                        }
                    }
            }
        }
        #if os(iOS)
        .navigationBarTitleDisplayMode(.large)
        #endif
        .alert("Delete Favorite?", isPresented: Binding(
            get: { isConfirmingDelete },
            set: { if !$0 { update(.active(.favorites(.workflow(.confirmDelete(.cancelled))))) } }
        )) {
            Button("Cancel", role: .cancel) {
                update(.active(.favorites(.workflow(.confirmDelete(.cancelled)))))
            }
            Button("Delete", role: .destructive) {
                update(.active(.favorites(.workflow(.confirmDelete(.confirmed)))))
            }
        }
    }

    private var favoritesList: some View {
        List {
            if model.favorites.isEmpty {
                ContentUnavailableView(
                    "No Favorites",
                    systemImage: "star",
                    description: Text("Add locations to see their weather at a glance.")
                )
            } else {
                ForEach(model.favorites, id: \.location) { favorite in
                    FavoriteCard(favorite: favorite)
                }
            }
        }
    }
}

#Preview {
    NavigationStack {
        FavoritesView(model: FavoritesViewModel(
            favorites: [
                FavoriteViewModel(name: "London", location: Location(lat: 51.5074, lon: -0.1278)),
                FavoriteViewModel(name: "Paris", location: Location(lat: 48.8566, lon: 2.3522))
            ],
            workflow: nil
        ))
    }
    .previewEnvironment()
}
