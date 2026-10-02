import 'dart:typed_data';

import 'bincode.dart';

// ---------------------------------------------------------------------------
// Events (Dart → Rust), order matches `shared::Event`
// ---------------------------------------------------------------------------

sealed class Event {
  void serialize(BincodeSerializer s);

  Uint8List toBytes() {
    final buf = BincodeSerializer();
    serialize(buf);
    return buf.toBytes();
  }
}

class EventIncrement extends Event {
  @override
  void serialize(BincodeSerializer s) => s.serializeVariantIndex(0);
}

class EventDecrement extends Event {
  @override
  void serialize(BincodeSerializer s) => s.serializeVariantIndex(1);
}

class EventReset extends Event {
  @override
  void serialize(BincodeSerializer s) => s.serializeVariantIndex(2);
}

// ---------------------------------------------------------------------------
// Requests / effects (Rust → Dart), order matches `shared::Effect`
// ---------------------------------------------------------------------------

class Request {
  final int id;
  final Effect effect;

  const Request(this.id, this.effect);

  static Request deserialize(BincodeDeserializer d) {
    final id = d.deserializeU32();
    final effect = Effect.deserialize(d);
    return Request(id, effect);
  }
}

List<Request> deserializeRequests(Uint8List bytes) {
  final d = BincodeDeserializer(bytes);
  final len = d.deserializeLen();
  return List.generate(len, (_) => Request.deserialize(d));
}

sealed class Effect {
  static Effect deserialize(BincodeDeserializer d) {
    final idx = d.deserializeVariantIndex();
    return switch (idx) {
      0 => EffectRender(),
      _ => throw ArgumentError('Unknown Effect variant $idx'),
    };
  }
}

class EffectRender extends Effect {}

// ---------------------------------------------------------------------------
// ViewModel (Rust → Dart)
// ---------------------------------------------------------------------------

class ViewModel {
  final String count;

  const ViewModel(this.count);

  factory ViewModel.deserialize(BincodeDeserializer d) {
    return ViewModel(d.deserializeStr());
  }
}
