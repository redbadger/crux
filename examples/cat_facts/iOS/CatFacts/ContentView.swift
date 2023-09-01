import SharedTypes
import SwiftUI


struct ContentView: View {
    @ObservedObject var core: Core

    init(core: Core) {
        self.core = core
        core.update(.get)
        core.update(.getPlatform)
    }

    var body: some View {
        VStack {
            Image(systemName: "globe")
                .imageScale(.large)
                .foregroundColor(.accentColor)
            Text(core.view.platform)
            core.view.image.map { image in
                AnyView(
                    // For the loading image to work properly, we'd need to add
                    // caching here
                    AsyncImage(url: URL(string: image.href)) { image in
                        image
                            .resizable()
                            .scaledToFit()
                    } placeholder: {
                        EmptyView()
                    }
                    .frame(maxHeight: 250)
                    .padding()
                )
            } ?? AnyView(EmptyView())
            Text(core.view.fact).padding()
            HStack {
                ActionButton(label: "Clear", color: .red) {
                    core.update(.clear)
                }
                ActionButton(label: "Get", color: .green) {
                    core.update(.get)
                }
                ActionButton(label: "Fetch", color: .yellow) {
                    core.update(.fetch)
                }
            }
        }
    }
}

struct ActionButton: View {
    var label: String
    var color: Color
    var action: () -> Void

    init(label: String, color: Color, action: @escaping () -> Void) {
        self.label = label
        self.color = color
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            Text(label)
                .fontWeight(.bold)
                .font(.body)
                .padding(EdgeInsets(top: 10, leading: 15, bottom: 10, trailing: 15))
                .background(color)
                .cornerRadius(10)
                .foregroundColor(.white)
                .padding()
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(core: Core())
    }
}
