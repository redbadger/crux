import SharedTypes
import SwiftUI

struct ContentView: View {
    @ObservedObject var core: Core
    
    init(core: Core) {
        self.core = core
        Task.init {
            while(true) {
                core.update(.tick)
                await Task.yield()
            }
        }
        
        Task.init {
            while(true) {
                try await Task.sleep(nanoseconds: UInt64(Double(NSEC_PER_SEC)))
                core.update(.newPeriod)
            }
        }
        
        core.update(.tick)
    }

    var body: some View {
        VStack {
            Text(String(core.view.count)).font(.largeTitle)
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(core: Core())
    }
}
