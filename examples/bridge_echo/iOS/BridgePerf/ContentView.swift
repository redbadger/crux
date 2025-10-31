import App
import SwiftUI

struct ContentView: View {
    @ObservedObject var core: Core
    @State private var isRunning = true
    @State private var tickTask: Task<Void, Never>? = nil
    @State private var periodTask: Task<Void, Never>? = nil

    private static let payloadSize = 10
    private let payload: [DataPoint] = (0..<Self.payloadSize).map { i in
        DataPoint(
            id: UInt64(i),
            value: Double.random(in: 0...1),
            label: "item_\(i)",
            metadata: Bool.random() ? "meta_\(i)" : nil
        )
    }

    init(core: Core) {
        self.core = core
    }

    private func startTasks() {
        tickTask = Task {
            while !Task.isCancelled {
                if isRunning {
                    core.update(.tick(payload))
                }
                await Task.yield()
            }
        }

        periodTask = Task {
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: UInt64(Double(NSEC_PER_SEC)))
                if isRunning {
                    core.update(.newPeriod)
                }
            }
        }
    }

    private func toggleRunning() {
        isRunning.toggle()
        if isRunning {
            startTasks()
        } else {
            core.update(.reset)
            tickTask?.cancel()
            periodTask?.cancel()
            tickTask = nil
            periodTask = nil
        }
    }

    var body: some View {
        VStack {
            Text(String(core.view.count))
                .font(.largeTitle)

            SparklineView(data: core.view.log)
                .frame(height: 200)
                .padding()
            let columns = [
                GridItem(alignment: .trailing),
                GridItem(alignment: .leading),
            ]
            LazyVGrid(columns: columns) {
                GridRow {
                    Text("Overall average:")
                    Text("\(core.view.average) /s")
                }
                GridRow {
                    Text("10 second moving average:")
                    Text("\(core.view.movingAverage) /s")
                }
                GridRow {
                    Text("Max:")
                    Text("\(core.view.max) /s")
                }
            }
            .font(.caption)
            .padding()
            .background(Color.white.opacity(0.8))
            .cornerRadius(8)

            Button(action: toggleRunning) {
                Text(isRunning ? "Stop" : "Start")
                    .padding()
                    .background(isRunning ? Color.red : Color.green)
                    .foregroundColor(.white)
                    .cornerRadius(8)
            }
            .padding()
        }
        .onAppear {
            startTasks()
        }
        .onDisappear {
            tickTask?.cancel()
            periodTask?.cancel()
        }
    }

}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(core: Core())
    }
}
