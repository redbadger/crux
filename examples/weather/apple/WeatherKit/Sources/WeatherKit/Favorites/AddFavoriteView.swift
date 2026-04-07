import App
import SwiftUI

struct AddFavoriteView: View {
    @Environment(CoreUpdater.self) var update
    let model: AddFavoriteViewModel

    var body: some View {
        VStack {
            // Search Bar
            HStack {
                Image(systemName: "magnifyingglass")
                    .foregroundColor(.secondary)
                TextField("Search location...", text: Binding(
                    get: { model.searchInput },
                    set: { update(.active(.favorites(.workflow(.add(.search($0)))))) }
                ))
                .textFieldStyle(RoundedBorderTextFieldStyle())

                if !model.searchInput.isEmpty {
                    Button {
                        update(.active(.favorites(.workflow(.add(.search(""))))))
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(.secondary)
                    }
                }
            }
            .padding()

            // Search Results
            if model.searching {
                ProgressView("Searching...")
                    .padding()
            } else if let results = model.searchResults {
                if results.isEmpty {
                    Text("No results found")
                        .foregroundColor(.secondary)
                        .padding()
                } else {
                    List(results, id: \.lat) { result in
                        Button {
                            update(.active(.favorites(.workflow(.add(.submit(result))))))
                        } label: {
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
                Text("Enter a location to search")
                    .foregroundColor(.secondary)
                    .padding()
            }

            Spacer()
        }
        .navigationTitle("Add Favorite")
        .toolbar {
            ToolbarItem(placement: .automatic) {
                Button("Cancel") {
                    update(.active(.favorites(.workflow(.add(.cancel)))))
                }
            }
        }
    }
}

#Preview {
    AddFavoriteView(model: AddFavoriteViewModel(
        searchInput: "",
        searchResults: nil,
        searching: false
    ))
    .previewEnvironment()
}
