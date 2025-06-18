import SwiftUI
import SharedTypes

// View modifier to handle smooth transitions
private struct SmoothTransitionModifier: ViewModifier {
    let workflow: WorkflowViewModel
    
    func body(content: Content) -> some View {
        content
            .animation(.easeInOut, value: workflow)
    }
}

struct ContentView: View {
    @ObservedObject var core: Core
    
    init(core: Core) {
        self.core = core
    }
    
    var body: some View {
        NavigationStack {
            ZStack {
                // Base background that's always present
                Color(.systemGroupedBackground)
                    .ignoresSafeArea()
                
                // Content views
                switch core.view.workflow {
                case .home:
                    HomeView(core: core)
                        .transition(
                            .opacity.combined(with: .offset(x: 0, y: 10))
                        )
                case .favorites, .confirmDeleteFavorite:
                    FavoritesView(core: core)
                        .transition(
                            .opacity.combined(with: .offset(x: 0, y: 10))
                        )
                case .addFavorite:
                    AddFavoriteView(core: core)
                        .transition(
                            .opacity.combined(with: .offset(x: 0, y: 10))
                        )
                }
            }
            .animation(.easeOut(duration: 0.2), value: core.view.workflow)
        }
    }
}

#Preview {
    ContentView(core: Core())
} 
