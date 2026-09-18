import 'dart:convert';
import 'dart:typed_data';

/// Bincode v1 (serde-compatible) — matches `bincode = "1.3"` in crux_core.
class BincodeSerializer {
  final _buf = BytesBuilder();

  void serializeU32(int v) {
    final bd = ByteData(4)..setUint32(0, v, Endian.little);
    _buf.add(bd.buffer.asUint8List());
  }

  void serializeU64(int v) {
    final bd = ByteData(8)..setUint64(0, v, Endian.little);
    _buf.add(bd.buffer.asUint8List());
  }

  void serializeVariantIndex(int idx) => serializeU32(idx);

  void serializeLen(int len) => serializeU64(len);

  void serializeStr(String v) {
    final bytes = utf8.encode(v);
    serializeLen(bytes.length);
    _buf.add(bytes);
  }

  Uint8List toBytes() => _buf.toBytes();
}

class BincodeDeserializer {
  final ByteData _data;
  int _offset = 0;

  BincodeDeserializer(Uint8List bytes)
    : _data = bytes.buffer.asByteData(bytes.offsetInBytes, bytes.length);

  int deserializeU32() {
    final v = _data.getUint32(_offset, Endian.little);
    _offset += 4;
    return v;
  }

  int deserializeU64() {
    final v = _data.getUint64(_offset, Endian.little);
    _offset += 8;
    return v;
  }

  int deserializeLen() => deserializeU64();

  int deserializeVariantIndex() => deserializeU32();

  String deserializeStr() {
    final len = deserializeLen();
    final bytes = Uint8List.view(
      _data.buffer,
      _data.offsetInBytes + _offset,
      len,
    );
    _offset += len;
    return utf8.decode(bytes);
  }
}
