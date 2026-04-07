import App
import SwiftUI

struct FavoritesView: View {
    @Environment(CoreUpdater.self) var update
    let model: FavoritesViewModel

    var body: some View {
        ZStack {
            Color.systemGroupedBackground
                .ignoresSafeArea()

            if case let .add(addModel) = model.workflow {
                AddFavoriteView(model: addModel)
            } else {
                VStack {
                    favoritesList
                }
                .overlay {
                    if case let .confirmDelete(location) = model.workflow {
                        deleteConfirmationOverlay(location: location)
                    }
                }
            }
        }
        .navigationTitle("Favorites")
        #if os(iOS)
        .navigationBarTitleDisplayMode(.large)
        #endif
        .toolbar {
            ToolbarItem(placement: .automatic) {
                Button {
                    update(.active(.favorites(.goToHome)))
                } label: {
                    Image(systemName: "chevron.left")
                    Text("Home")
                }
            }
            ToolbarItem(placement: .automatic) {
                Button {
                    update(.active(.favorites(.requestAddFavorite)))
                } label: {
                    Image(systemName: "plus")
                }
            }
        }
    }

    private var favoritesList: some View {
        ScrollView {
            VStack(spacing: 24) {
                if model.favorites.isEmpty {
                    Text("No favorites yet")
                        .foregroundColor(.secondary)
                        .padding()
                } else {
                    ForEach(model.favorites, id: \.location) { favorite in
                        FavoriteCard(favorite: favorite)
                    }
                }
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical)
        }
    }

    private func deleteConfirmationOverlay(location: Location) -> some View {
        ZStack {
            Color.black.opacity(0.4)
                .ignoresSafeArea()

            GroupBox {
                VStack(spacing: 20) {
                    Text("Delete Favorite?")
                        .font(.headline)

                    HStack(spacing: 20) {
                        Button("Cancel") {
                            update(.active(.favorites(.workflow(.confirmDelete(.cancelled)))))
                        }
                        .buttonStyle(.bordered)

                        Button("Delete") {
                            update(.active(.favorites(.workflow(.confirmDelete(.confirmed)))))
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.red)
                    }
                }
            }
            .padding()
        }
    }
}

#Preview {
    FavoritesView(model: FavoritesViewModel(
        favorites: [
            FavoriteViewModel(name: "London", location: Location(lat: 51.5074, lon: -0.1278)),
            FavoriteViewModel(name: "Paris", location: Location(lat: 48.8566, lon: 2.3522))
        ],
        workflow: nil
    ))
    .previewEnvironment()
}
