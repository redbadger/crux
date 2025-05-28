import SwiftUI
import SharedTypes

struct ContentView: View {
    @ObservedObject var core: Core
    
    init(core: Core) {
        self.core = core
    }
    
    var body: some View {
        Group {
            switch core.view.workflow {
            case .home:
                HomeView(core: core)
            case .favorites:
                FavoritesView(core: core)
            case .addFavorite:
                AddFavoriteView(core: core)
            case .confirmDeleteFavorite(let lat, let lng):
                FavoritesView(core: core)
                    .overlay(
                        DeleteConfirmationView(lat: lat, lng: lng, core: core)
                            .background(Color.black.opacity(0.4))
                            .edgesIgnoringSafeArea(.all)
                    )
            }
        }
    }
}

#Preview {
    ContentView(core: Core())
} 
