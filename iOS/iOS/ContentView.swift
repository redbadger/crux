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


struct ContentView: View {
    var body: some View {
        VStack {
            Image(systemName: "globe")
                .imageScale(.large)
                .foregroundColor(.accentColor)
            Text(try! addForPlatform(1, 2, GetPlatform()))
        }
        .padding()
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}
