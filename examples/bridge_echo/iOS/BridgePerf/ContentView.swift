import SharedTypes
import SwiftUI

struct ContentView: View {
    @ObservedObject var model: Core
    
    init(model: Core) {
        self.model = model
        Task.init {
            while(true) {
                model.update(event: .tick)
                await Task.yield()
            }
        }
        
        Task.init {
            while(true) {
                try await Task.sleep(nanoseconds: UInt64(Double(NSEC_PER_SEC)))
                model.update(event: .newPeriod)
            }
        }
        
        model.update(event: .tick)
    }

    var body: some View {
        VStack {
            Text(String(model.view.count)).font(.largeTitle)
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(model: Core())
    }
}
