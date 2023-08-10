//
//  Core.swift
//  TapToPay
//
//  Created by Viktor Charypar on 19/04/2023.
//

import Serde
import SwiftUI
import SharedTypes

@MainActor
class Core: ObservableObject {
    @Published var view = ViewModel(screen: .payment(Payment(amount: 0, status: .new)))

    func update(event: Event) {
        let reqs: [Request] = try! [Request].bincodeDeserialize(input: TapToPay.processEvent(try! event.bincodeSerialize()))

        for req in reqs {
            process_effect(request: req)
        }
    }

    func process_effect(request: Request) {
        switch request.effect {
        case .render(_):
            view = try! ViewModel.bincodeDeserialize(input: TapToPay.view())
        case .delay(.start(millis: let ms)):
            Task.init {
                try? await Task.sleep(for: .milliseconds(Double(ms)))

                let effects = TapToPay.handleResponse(request.uuid,  [])

                let reqs: [Request] = try! [Request].bincodeDeserialize(input: effects)
                for req in reqs {
                    process_effect(request: req)
                }
            }
        }
    }
}

