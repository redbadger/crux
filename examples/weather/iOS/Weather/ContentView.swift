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
            case .favorites, .confirmDeleteFavorite:
                FavoritesView(core: core)
            case .addFavorite:
                AddFavoriteView(core: core)
            }
        }
    }
}

#Preview {
    ContentView(core: Core())
} 
