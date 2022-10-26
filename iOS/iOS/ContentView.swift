//
//  ContentView.swift
//  iOS
//
//  Created by Stuart Harris on 08/10/2022.
//

import SwiftUI

class GetPlatform: Platform {
    func get() -> String {
        return UIDevice.current.systemName + " " + UIDevice.current.systemVersion
    }
}

extension CatFactData: Decodable {
    enum CodingKeys: String, CodingKey {
        case fact
        case length
    }

    public init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        fact = try values.decode(String.self, forKey: .fact)
        length = try values.decode(Int32.self, forKey: .length)
    }
}

@MainActor
class Model: ObservableObject {
    @Published var fact = CatFact(CatFactData(fact: "", length: 0))

    init() {
        getFact()
    }

    private func getFact() {
        Task {
            let (data, _) = try! await URLSession.shared.data(from: URL(string: "https://catfact.ninja/fact")!)
            let decodedResponse = try? JSONDecoder().decode(CatFactData.self, from: data)
            fact = CatFact(decodedResponse!)
        }
    }
}

struct ContentView: View {
    @ObservedObject var model: Model

    var body: some View {
        VStack {
            Image(systemName: "globe")
                .imageScale(.large)
                .foregroundColor(.accentColor)
            Text(try! addForPlatform(1, 2, GetPlatform()))
            Text(model.fact.format())
        }
        .padding()
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(model: Model())
    }
}
