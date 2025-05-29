import SwiftUI
import SharedTypes

struct FavoritesView: View {
    @ObservedObject var core: Core
    
    init(core: Core) {
        self.core = core
        core.update(.favorites(.restore))
    }
    
    var body: some View {
        NavigationView {
            ZStack {
                // Main content
                ScrollView {
                    VStack(spacing: 24) {
                        if case .favorites(let favorites) = core.view.workflow {
                            if favorites.isEmpty {
                                Text("No favorites yet")
                                    .foregroundColor(.secondary)
                                    .padding()
                            } else {
                                ForEach(favorites, id: \.lat) { favorite in
                                    FavoriteCard(favorite: favorite, core: core)
                                }
                            }
                        } else if case .confirmDeleteFavorite(let lat, let lng) = core.view.workflow {

                        } else {
                            Text("Not in favorites view")
                                .foregroundColor(.secondary)
                        }
                    }
                    .padding(.vertical)
                }
                .background(Color(.systemGroupedBackground))
                .navigationTitle("Favorites")
                .toolbar {
                    ToolbarItem(placement: .navigationBarLeading) {
                        Button(action: {
                            core.update(.navigate(Workflow.home))
                        }) {
                            Image(systemName: "chevron.left")
                            Text("Home")
                        }
                    }
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button(action: {
                            core.update(.navigate(.addFavorite))
                        }) {
                            Image(systemName: "plus")
                        }
                    }
                }

                // Overlay
                if case .confirmDeleteFavorite(let lat, let lng) = core.view.workflow {
                    Color.black.opacity(0.4)
                        .ignoresSafeArea()
                    DeleteConfirmationView(lat: lat, lng: lng, core: core)
                }
            }
        }
    }
}

struct FavoriteCard: View {
    let favorite: SharedTypes.FavoriteView
    let core: Core
    
    var body: some View {
        Button(action: {
            core.update(.home(.show(favorite.lat, favorite.lon)))
        }) {
            HStack {
                VStack(alignment: .leading) {
                    Text(favorite.name)
                        .font(.headline)
                    if let summary = favorite.summary {
                        Text(summary)
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                }
                
                Spacer()
                
                Button(action: {
                    let geo = GeocodingResponse(
                        name: favorite.name,
                        local_names: nil,
                        lat: favorite.lat,
                        lon: favorite.lon,
                        country: "",
                        state: nil
                    )
                    core.update(.favorites(.deletePressed(Favorite(geo: geo, current: nil))))
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
    let lng: Double
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
