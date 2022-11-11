
import { Serializer, Deserializer } from '../serde/mod.ts';
import { BcsSerializer, BcsDeserializer } from '../bcs/mod.ts';
import { Optional, Seq, Tuple, ListTuple, unit, bool, int8, int16, int32, int64, int128, uint8, uint16, uint32, uint64, uint128, float32, float64, char, str, bytes } from '../serde/mod.ts';

export class CatImage {

constructor (public file: str) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeStr(this.file);
}

static deserialize(deserializer: Deserializer): CatImage {
  const file = deserializer.deserializeStr();
  return new CatImage(file);
}

}
export abstract class Msg {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): Msg {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return MsgVariantNone.load(deserializer);
    case 1: return MsgVariantGetPlatform.load(deserializer);
    case 2: return MsgVariantPlatform.load(deserializer);
    case 3: return MsgVariantClear.load(deserializer);
    case 4: return MsgVariantGet.load(deserializer);
    case 5: return MsgVariantFetch.load(deserializer);
    case 6: return MsgVariantRestore.load(deserializer);
    case 7: return MsgVariantSetState.load(deserializer);
    case 8: return MsgVariantSetFact.load(deserializer);
    case 9: return MsgVariantSetImage.load(deserializer);
    case 10: return MsgVariantCurrentTime.load(deserializer);
    default: throw new Error("Unknown variant index for Msg: " + index);
  }
}
}


export class MsgVariantNone extends Msg {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
}

static load(deserializer: Deserializer): MsgVariantNone {
  return new MsgVariantNone();
}

}

export class MsgVariantGetPlatform extends Msg {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
}

static load(deserializer: Deserializer): MsgVariantGetPlatform {
  return new MsgVariantGetPlatform();
}

}

export class MsgVariantPlatform extends Msg {

constructor (public value: PlatformMsg) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): MsgVariantPlatform {
  const value = PlatformMsg.deserialize(deserializer);
  return new MsgVariantPlatform(value);
}

}

export class MsgVariantClear extends Msg {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(3);
}

static load(deserializer: Deserializer): MsgVariantClear {
  return new MsgVariantClear();
}

}

export class MsgVariantGet extends Msg {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(4);
}

static load(deserializer: Deserializer): MsgVariantGet {
  return new MsgVariantGet();
}

}

export class MsgVariantFetch extends Msg {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(5);
}

static load(deserializer: Deserializer): MsgVariantFetch {
  return new MsgVariantFetch();
}

}

export class MsgVariantRestore extends Msg {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(6);
}

static load(deserializer: Deserializer): MsgVariantRestore {
  return new MsgVariantRestore();
}

}

export class MsgVariantSetState extends Msg {

constructor (public value: Optional<Seq<uint8>>) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(7);
  Helpers.serializeOptionVectorU8(this.value, serializer);
}

static load(deserializer: Deserializer): MsgVariantSetState {
  const value = Helpers.deserializeOptionVectorU8(deserializer);
  return new MsgVariantSetState(value);
}

}

export class MsgVariantSetFact extends Msg {

constructor (public value: Seq<uint8>) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(8);
  Helpers.serializeVectorU8(this.value, serializer);
}

static load(deserializer: Deserializer): MsgVariantSetFact {
  const value = Helpers.deserializeVectorU8(deserializer);
  return new MsgVariantSetFact(value);
}

}

export class MsgVariantSetImage extends Msg {

constructor (public value: Seq<uint8>) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(9);
  Helpers.serializeVectorU8(this.value, serializer);
}

static load(deserializer: Deserializer): MsgVariantSetImage {
  const value = Helpers.deserializeVectorU8(deserializer);
  return new MsgVariantSetImage(value);
}

}

export class MsgVariantCurrentTime extends Msg {

constructor (public value: str) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(10);
  serializer.serializeStr(this.value);
}

static load(deserializer: Deserializer): MsgVariantCurrentTime {
  const value = deserializer.deserializeStr();
  return new MsgVariantCurrentTime(value);
}

}
export abstract class PlatformMsg {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): PlatformMsg {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return PlatformMsgVariantGet.load(deserializer);
    case 1: return PlatformMsgVariantSet.load(deserializer);
    default: throw new Error("Unknown variant index for PlatformMsg: " + index);
  }
}
}


export class PlatformMsgVariantGet extends PlatformMsg {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
}

static load(deserializer: Deserializer): PlatformMsgVariantGet {
  return new PlatformMsgVariantGet();
}

}

export class PlatformMsgVariantSet extends PlatformMsg {

constructor (public value: str) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  serializer.serializeStr(this.value);
}

static load(deserializer: Deserializer): PlatformMsgVariantSet {
  const value = deserializer.deserializeStr();
  return new PlatformMsgVariantSet(value);
}

}
export class Request {

constructor (public uuid: Seq<uint8>, public body: RequestBody) {
}

public serialize(serializer: Serializer): void {
  Helpers.serializeVectorU8(this.uuid, serializer);
  this.body.serialize(serializer);
}

static deserialize(deserializer: Deserializer): Request {
  const uuid = Helpers.deserializeVectorU8(deserializer);
  const body = RequestBody.deserialize(deserializer);
  return new Request(uuid,body);
}

}
export abstract class RequestBody {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): RequestBody {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return RequestBodyVariantTime.load(deserializer);
    case 1: return RequestBodyVariantHttp.load(deserializer);
    case 2: return RequestBodyVariantPlatform.load(deserializer);
    case 3: return RequestBodyVariantKVRead.load(deserializer);
    case 4: return RequestBodyVariantKVWrite.load(deserializer);
    case 5: return RequestBodyVariantRender.load(deserializer);
    default: throw new Error("Unknown variant index for RequestBody: " + index);
  }
}
}


export class RequestBodyVariantTime extends RequestBody {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
}

static load(deserializer: Deserializer): RequestBodyVariantTime {
  return new RequestBodyVariantTime();
}

}

export class RequestBodyVariantHttp extends RequestBody {

constructor (public value: str) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  serializer.serializeStr(this.value);
}

static load(deserializer: Deserializer): RequestBodyVariantHttp {
  const value = deserializer.deserializeStr();
  return new RequestBodyVariantHttp(value);
}

}

export class RequestBodyVariantPlatform extends RequestBody {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
}

static load(deserializer: Deserializer): RequestBodyVariantPlatform {
  return new RequestBodyVariantPlatform();
}

}

export class RequestBodyVariantKVRead extends RequestBody {

constructor (public value: str) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(3);
  serializer.serializeStr(this.value);
}

static load(deserializer: Deserializer): RequestBodyVariantKVRead {
  const value = deserializer.deserializeStr();
  return new RequestBodyVariantKVRead(value);
}

}

export class RequestBodyVariantKVWrite extends RequestBody {

constructor (public field0: str, public field1: Seq<uint8>) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(4);
  serializer.serializeStr(this.field0);
  Helpers.serializeVectorU8(this.field1, serializer);
}

static load(deserializer: Deserializer): RequestBodyVariantKVWrite {
  const field0 = deserializer.deserializeStr();
  const field1 = Helpers.deserializeVectorU8(deserializer);
  return new RequestBodyVariantKVWrite(field0,field1);
}

}

export class RequestBodyVariantRender extends RequestBody {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(5);
}

static load(deserializer: Deserializer): RequestBodyVariantRender {
  return new RequestBodyVariantRender();
}

}
export class Response {

constructor (public uuid: Seq<uint8>, public body: ResponseBody) {
}

public serialize(serializer: Serializer): void {
  Helpers.serializeVectorU8(this.uuid, serializer);
  this.body.serialize(serializer);
}

static deserialize(deserializer: Deserializer): Response {
  const uuid = Helpers.deserializeVectorU8(deserializer);
  const body = ResponseBody.deserialize(deserializer);
  return new Response(uuid,body);
}

}
export abstract class ResponseBody {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): ResponseBody {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return ResponseBodyVariantHttp.load(deserializer);
    case 1: return ResponseBodyVariantTime.load(deserializer);
    case 2: return ResponseBodyVariantPlatform.load(deserializer);
    case 3: return ResponseBodyVariantKVRead.load(deserializer);
    case 4: return ResponseBodyVariantKVWrite.load(deserializer);
    default: throw new Error("Unknown variant index for ResponseBody: " + index);
  }
}
}


export class ResponseBodyVariantHttp extends ResponseBody {

constructor (public value: Seq<uint8>) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  Helpers.serializeVectorU8(this.value, serializer);
}

static load(deserializer: Deserializer): ResponseBodyVariantHttp {
  const value = Helpers.deserializeVectorU8(deserializer);
  return new ResponseBodyVariantHttp(value);
}

}

export class ResponseBodyVariantTime extends ResponseBody {

constructor (public value: str) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  serializer.serializeStr(this.value);
}

static load(deserializer: Deserializer): ResponseBodyVariantTime {
  const value = deserializer.deserializeStr();
  return new ResponseBodyVariantTime(value);
}

}

export class ResponseBodyVariantPlatform extends ResponseBody {

constructor (public value: str) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
  serializer.serializeStr(this.value);
}

static load(deserializer: Deserializer): ResponseBodyVariantPlatform {
  const value = deserializer.deserializeStr();
  return new ResponseBodyVariantPlatform(value);
}

}

export class ResponseBodyVariantKVRead extends ResponseBody {

constructor (public value: Optional<Seq<uint8>>) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(3);
  Helpers.serializeOptionVectorU8(this.value, serializer);
}

static load(deserializer: Deserializer): ResponseBodyVariantKVRead {
  const value = Helpers.deserializeOptionVectorU8(deserializer);
  return new ResponseBodyVariantKVRead(value);
}

}

export class ResponseBodyVariantKVWrite extends ResponseBody {

constructor (public value: bool) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(4);
  serializer.serializeBool(this.value);
}

static load(deserializer: Deserializer): ResponseBodyVariantKVWrite {
  const value = deserializer.deserializeBool();
  return new ResponseBodyVariantKVWrite(value);
}

}
export class ViewModel {

constructor (public fact: str, public image: Optional<CatImage>, public platform: str) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeStr(this.fact);
  Helpers.serializeOptionCatImage(this.image, serializer);
  serializer.serializeStr(this.platform);
}

static deserialize(deserializer: Deserializer): ViewModel {
  const fact = deserializer.deserializeStr();
  const image = Helpers.deserializeOptionCatImage(deserializer);
  const platform = deserializer.deserializeStr();
  return new ViewModel(fact,image,platform);
}

}
export class Helpers {
  static serializeOptionCatImage(value: Optional<CatImage>, serializer: Serializer): void {
    if (value) {
        serializer.serializeOptionTag(true);
        value.serialize(serializer);
    } else {
        serializer.serializeOptionTag(false);
    }
  }

  static deserializeOptionCatImage(deserializer: Deserializer): Optional<CatImage> {
    const tag = deserializer.deserializeOptionTag();
    if (!tag) {
        return null;
    } else {
        return CatImage.deserialize(deserializer);
    }
  }

  static serializeOptionVectorU8(value: Optional<Seq<uint8>>, serializer: Serializer): void {
    if (value) {
        serializer.serializeOptionTag(true);
        Helpers.serializeVectorU8(value, serializer);
    } else {
        serializer.serializeOptionTag(false);
    }
  }

  static deserializeOptionVectorU8(deserializer: Deserializer): Optional<Seq<uint8>> {
    const tag = deserializer.deserializeOptionTag();
    if (!tag) {
        return null;
    } else {
        return Helpers.deserializeVectorU8(deserializer);
    }
  }

  static serializeVectorU8(value: Seq<uint8>, serializer: Serializer): void {
    serializer.serializeLen(value.length);
    value.forEach((item: uint8) => {
        serializer.serializeU8(item);
    });
  }

  static deserializeVectorU8(deserializer: Deserializer): Seq<uint8> {
    const length = deserializer.deserializeLen();
    const list: Seq<uint8> = [];
    for (let i = 0; i < length; i++) {
        list.push(deserializer.deserializeU8());
    }
    return list;
  }

}

