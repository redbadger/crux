import SwiftUI
import App

struct FavoritesView: View {
    @ObservedObject var core: Core
    
    init(core: Core) {
        self.core = core
        core.update(.favorites(.restore))
    }
    
    var body: some View {
        NavigationView {
            ZStack {
                Color(.systemGroupedBackground)
                    .ignoresSafeArea()
                
                VStack {
                    switch core.view.workflow {
                    case .favorites(let favorites, let deleteConfirmation):
                        if let deleteConfirmation {
                            let lat = deleteConfirmation.lat
                            let lon = deleteConfirmation.lon

                            favoritesList(favorites)
                                .overlay(deleteConfirmationOverlay(lat: lat, lon: lon))
                        } else {
                            favoritesList(favorites)
                        }



                    default:
                        Spacer()
                        Text("No favorites yet")
                            .foregroundColor(.secondary)
                        Spacer()
                    }
                }
            }
            .navigationTitle("Favorites")
            .navigationBarTitleDisplayMode(.large)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button(action: { core.update(.navigate(Workflow.home)) }) {
                        Image(systemName: "chevron.left")
                        Text("Home")
                    }
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: { core.update(.navigate(.addFavorite)) }) {
                        Image(systemName: "plus")
                    }
                }
            }
        }
    }
    
    private func favoritesList(_ favorites: [FavoriteView]) -> some View {
        ScrollView {
            VStack(spacing: 24) {
                if favorites.isEmpty {
                    Text("No favorites yet")
                        .foregroundColor(.secondary)
                        .padding()
                } else {
                    ForEach(favorites, id: \.location) { favorite in
                        FavoriteCard(favorite: favorite, core: core)
                    }
                }
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical)
        }
    }
    
    private func deleteConfirmationOverlay(lat: Double, lon: Double) -> some View {
        ZStack {
            Color.black.opacity(0.4)
                .ignoresSafeArea()
            DeleteConfirmationView(lat: lat, lon: lon, core: core)
        }
    }
}

struct FavoriteCard: View {
    let favorite: FavoriteView
    let core: Core
    
    var body: some View {
        Button(action: {
            core.update(.home(.show))
        }) {
            HStack {
                VStack(alignment: .leading) {
                    Text(favorite.name)
                        .font(.headline)
                }
                
                Spacer()
                
                Button(action: {
                    core.update(.favorites(.deletePressed(favorite.location)))
                }) {
                    Image(systemName: "trash")
                        .foregroundColor(.red)
                }
            }
            .padding()
            .background(Color(.systemBackground))
            .cornerRadius(12)
            .shadow(radius: 2)
        }
        .buttonStyle(PlainButtonStyle())
        .padding(.horizontal)
    }
}

struct DeleteConfirmationView: View {
    let lat: Double
    let lon: Double
    let core: Core
    
    var body: some View {
        VStack(spacing: 20) {
            Text("Delete Favorite?")
                .font(.headline)
            
            HStack(spacing: 20) {
                Button("Cancel") {
                    core.update(.favorites(.deleteCancelled))
                }
                .buttonStyle(.bordered)
                
                Button("Delete") {
                    core.update(.favorites(.deleteConfirmed))
                }
                .buttonStyle(.borderedProminent)
                .tint(.red)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(radius: 2)
        .padding()
    }
}

#Preview {
    FavoritesView(core: Core())
} 
