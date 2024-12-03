import SwiftUI

struct SparklineView: View {
    var data: [UInt64]
    @State private var currentIndex: Int? = nil

    var body: some View {
        GeometryReader { geometry in
            let height = geometry.size.height
            let width = geometry.size.width
            let maxData = data.max() ?? 1
            let scale = height / CGFloat(maxData)

            let path = Path { path in
                guard let firstPoint = data.first else { return }
                path.move(to: CGPoint(x: 0, y: height - CGFloat(firstPoint) * scale))

                for (index, value) in data.enumerated() {
                    let x = width * CGFloat(index) / CGFloat(data.count - 1)
                    let y = height - CGFloat(value) * scale
                    path.addLine(to: CGPoint(x: x, y: y))
                }
            }

            path.stroke(Color.blue, lineWidth: 2)
                .background(
                    Color.clear
                        .contentShape(Rectangle())
                        .gesture(
                            DragGesture(minimumDistance: 0)
                                .onChanged { value in
                                    let index = Int((value.location.x / width) * CGFloat(data.count))
                                    if index >= 0 && index < data.count {
                                        currentIndex = index
                                    }
                                }
                        )
                )
                .overlay(
                    Group {
                        if let index = currentIndex, index < data.count {
                            let x = width * CGFloat(index) / CGFloat(data.count - 1)
                            let y = height - CGFloat(data[index]) * scale
                            Text("\(data[index])")
                                .font(.caption)
                                .padding(5)
                                .background(Color.white.opacity(0.8))
                                .cornerRadius(5)
                                .position(x: x, y: y - 20)
                        }
                    }
                )
        }
    }
}

struct SparklineView_Previews: PreviewProvider {
    static var previews: some View {
        SparklineView(data: [10, 20, 15, 30, 25, 40, 35])
            .frame(height: 200)
            .padding()
    }
}
