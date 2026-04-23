import App
import SwiftUI

struct FavoriteCard: View {
    @Environment(CoreUpdater.self) var update
    let favorite: FavoriteViewModel

    var body: some View {
        GroupBox {
            HStack {
                Text(favorite.name)
                    .font(.headline)

                Spacer()

                Button {
                    update(.active(.favorites(.requestDelete(favorite.location))))
                } label: {
                    Image(systemName: "trash")
                        .foregroundStyle(.red)
                }
            }
        }
        .padding(.horizontal)
    }
}

#Preview {
    FavoriteCard(favorite: FavoriteViewModel(
        name: "London",
        location: Location(lat: 51.5074, lon: -0.1278)
    ))
    .previewEnvironment()
}
