import 'package:flutter/foundation.dart';

import 'package:counter_flutter/src/rust/api/core.dart';

import 'bincode.dart';
import 'types.dart';

/// Flutter shell bridge — same flow as `Core.kt` on Android.
class Core extends ChangeNotifier {
  final CruxCore _core;

  ViewModel _viewModel;
  ViewModel get viewModel => _viewModel;

  Core() : _core = CruxCore(), _viewModel = const ViewModel('Count is: 0');

  Future<void> update(Event event) async {
    final effectsBytes = await _core.update(data: event.toBytes());
    await _handleEffects(effectsBytes);
  }

  Future<void> _handleEffects(Uint8List effectsBytes) async {
    final requests = deserializeRequests(effectsBytes);
    for (final request in requests) {
      await _processRequest(request);
    }
  }

  Future<void> _processRequest(Request request) async {
    switch (request.effect) {
      case EffectRender():
        await _render();
    }
  }

  Future<void> _render() async {
    final bytes = await _core.view();
    _viewModel = ViewModel.deserialize(BincodeDeserializer(bytes));
    notifyListeners();
  }
}
