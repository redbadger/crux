import SwiftUI
import App

struct AddFavoriteView: View {
    @ObservedObject var core: Core
    @State private var searchText = ""
    @State private var isSearching = false
    
    var body: some View {
        NavigationView {
            ZStack {
                Color(.systemGroupedBackground)
                    .ignoresSafeArea()
                VStack {
                    // Search Bar
                    HStack {
                        Image(systemName: "magnifyingglass")
                            .foregroundColor(.secondary)
                        TextField("Search location...", text: $searchText)
                            .textFieldStyle(RoundedBorderTextFieldStyle())
                            .onChange(of: searchText) { oldValue, newValue in
                                if !newValue.isEmpty {
                                    isSearching = true
                                    core.update(.favorites(.search(newValue)))
                                } else {
                                    isSearching = false
                                }
                            }
                        if !searchText.isEmpty {
                            Button(action: {
                                searchText = ""
                                isSearching = false
                            }) {
                                Image(systemName: "xmark.circle.fill")
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                    .padding()
                    
                    // Search Results
                    if case .addFavorite(let results) = core.view.workflow {
                        if isSearching {
                            if let results = results {
                                if results.isEmpty {
                                    Text("No results found")
                                        .foregroundColor(.secondary)
                                        .padding()
                                } else {
                                    List(results, id: \.lat) { result in
                                        Button(action: {
                                            core.update(.favorites(.submit(result)))
                                        }) {
                                            VStack(alignment: .leading) {
                                                Text(result.name)
                                                    .font(.headline)
                                                if let state = result.state {
                                                    Text("\(state), \(result.country)")
                                                        .font(.subheadline)
                                                        .foregroundColor(.secondary)
                                                } else {
                                                    Text(result.country)
                                                        .font(.subheadline)
                                                        .foregroundColor(.secondary)
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                ProgressView("Searching...")
                                    .padding()
                            }
                        } else {
                            Text("Enter a location to search")
                                .foregroundColor(.secondary)
                                .padding()
                        }
                    }
                    
                    Spacer()
                }
            }
            .navigationTitle("Add Favorite")
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
//                        core.update(.addFavorite(.cancel))
                        core.update(.navigate(Workflow.favorites(FavoritesState.idle)))
                    }
                }
            }
        }
    }
}

#Preview {
    AddFavoriteView(core: Core())
} 
