
import { Serializer, Deserializer } from '../serde/mod';

import { Optional, Seq, Tuple, ListTuple, unit, bool, int8, int16, int32, int64, int128, uint8, uint16, uint32, uint64, uint128, float32, float64, char, str, bytes } from '../serde/mod';

export abstract class Effect {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): Effect {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return EffectVariantRender.load(deserializer);
    case 1: return EffectVariantHttp.load(deserializer);
    case 2: return EffectVariantServerSentEvents.load(deserializer);
    default: throw new Error("Unknown variant index for Effect: " + index);
  }
}
}


export class EffectVariantRender extends Effect {

constructor (public value: RenderOperation) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): EffectVariantRender {
  const value = RenderOperation.deserialize(deserializer);
  return new EffectVariantRender(value);
}

}

export class EffectVariantHttp extends Effect {

constructor (public value: HttpRequest) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): EffectVariantHttp {
  const value = HttpRequest.deserialize(deserializer);
  return new EffectVariantHttp(value);
}

}

export class EffectVariantServerSentEvents extends Effect {

constructor (public value: SseRequest) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): EffectVariantServerSentEvents {
  const value = SseRequest.deserialize(deserializer);
  return new EffectVariantServerSentEvents(value);
}

}
export abstract class Event {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): Event {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return EventVariantGet.load(deserializer);
    case 1: return EventVariantIncrement.load(deserializer);
    case 2: return EventVariantDecrement.load(deserializer);
    case 3: return EventVariantStartWatch.load(deserializer);
    default: throw new Error("Unknown variant index for Event: " + index);
  }
}
}


export class EventVariantGet extends Event {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
}

static load(deserializer: Deserializer): EventVariantGet {
  return new EventVariantGet();
}

}

export class EventVariantIncrement extends Event {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
}

static load(deserializer: Deserializer): EventVariantIncrement {
  return new EventVariantIncrement();
}

}

export class EventVariantDecrement extends Event {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
}

static load(deserializer: Deserializer): EventVariantDecrement {
  return new EventVariantDecrement();
}

}

export class EventVariantStartWatch extends Event {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(3);
}

static load(deserializer: Deserializer): EventVariantStartWatch {
  return new EventVariantStartWatch();
}

}
export abstract class HttpError {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): HttpError {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return HttpErrorVariantUrl.load(deserializer);
    case 1: return HttpErrorVariantIo.load(deserializer);
    case 2: return HttpErrorVariantTimeout.load(deserializer);
    default: throw new Error("Unknown variant index for HttpError: " + index);
  }
}
}


export class HttpErrorVariantUrl extends HttpError {

constructor (public value: str) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  serializer.serializeStr(this.value);
}

static load(deserializer: Deserializer): HttpErrorVariantUrl {
  const value = deserializer.deserializeStr();
  return new HttpErrorVariantUrl(value);
}

}

export class HttpErrorVariantIo extends HttpError {

constructor (public value: str) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  serializer.serializeStr(this.value);
}

static load(deserializer: Deserializer): HttpErrorVariantIo {
  const value = deserializer.deserializeStr();
  return new HttpErrorVariantIo(value);
}

}

export class HttpErrorVariantTimeout extends HttpError {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
}

static load(deserializer: Deserializer): HttpErrorVariantTimeout {
  return new HttpErrorVariantTimeout();
}

}
export class HttpHeader {

constructor (public name: str, public value: str) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeStr(this.name);
  serializer.serializeStr(this.value);
}

static deserialize(deserializer: Deserializer): HttpHeader {
  const name = deserializer.deserializeStr();
  const value = deserializer.deserializeStr();
  return new HttpHeader(name,value);
}

}
export class HttpRequest {

constructor (public method: str, public url: str, public headers: Seq<HttpHeader>, public body: bytes) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeStr(this.method);
  serializer.serializeStr(this.url);
  Helpers.serializeVectorHttpHeader(this.headers, serializer);
  serializer.serializeBytes(this.body);
}

static deserialize(deserializer: Deserializer): HttpRequest {
  const method = deserializer.deserializeStr();
  const url = deserializer.deserializeStr();
  const headers = Helpers.deserializeVectorHttpHeader(deserializer);
  const body = deserializer.deserializeBytes();
  return new HttpRequest(method,url,headers,body);
}

}
export class HttpResponse {

constructor (public status: uint16, public headers: Seq<HttpHeader>, public body: bytes) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeU16(this.status);
  Helpers.serializeVectorHttpHeader(this.headers, serializer);
  serializer.serializeBytes(this.body);
}

static deserialize(deserializer: Deserializer): HttpResponse {
  const status = deserializer.deserializeU16();
  const headers = Helpers.deserializeVectorHttpHeader(deserializer);
  const body = deserializer.deserializeBytes();
  return new HttpResponse(status,headers,body);
}

}
export abstract class HttpResult {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): HttpResult {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return HttpResultVariantOk.load(deserializer);
    case 1: return HttpResultVariantErr.load(deserializer);
    default: throw new Error("Unknown variant index for HttpResult: " + index);
  }
}
}


export class HttpResultVariantOk extends HttpResult {

constructor (public value: HttpResponse) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): HttpResultVariantOk {
  const value = HttpResponse.deserialize(deserializer);
  return new HttpResultVariantOk(value);
}

}

export class HttpResultVariantErr extends HttpResult {

constructor (public value: HttpError) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
  this.value.serialize(serializer);
}

static load(deserializer: Deserializer): HttpResultVariantErr {
  const value = HttpError.deserialize(deserializer);
  return new HttpResultVariantErr(value);
}

}
export class RenderOperation {
constructor () {
}

public serialize(serializer: Serializer): void {
}

static deserialize(deserializer: Deserializer): RenderOperation {
  return new RenderOperation();
}

}
export class Request {

constructor (public id: uint32, public effect: Effect) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeU32(this.id);
  this.effect.serialize(serializer);
}

static deserialize(deserializer: Deserializer): Request {
  const id = deserializer.deserializeU32();
  const effect = Effect.deserialize(deserializer);
  return new Request(id,effect);
}

}
export class SseRequest {

constructor (public url: str) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeStr(this.url);
}

static deserialize(deserializer: Deserializer): SseRequest {
  const url = deserializer.deserializeStr();
  return new SseRequest(url);
}

}
export abstract class SseResponse {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): SseResponse {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return SseResponseVariantChunk.load(deserializer);
    case 1: return SseResponseVariantDone.load(deserializer);
    default: throw new Error("Unknown variant index for SseResponse: " + index);
  }
}
}


export class SseResponseVariantChunk extends SseResponse {

constructor (public value: Seq<uint8>) {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
  Helpers.serializeVectorU8(this.value, serializer);
}

static load(deserializer: Deserializer): SseResponseVariantChunk {
  const value = Helpers.deserializeVectorU8(deserializer);
  return new SseResponseVariantChunk(value);
}

}

export class SseResponseVariantDone extends SseResponse {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(1);
}

static load(deserializer: Deserializer): SseResponseVariantDone {
  return new SseResponseVariantDone();
}

}
export class ViewModel {

constructor (public text: str, public confirmed: bool) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeStr(this.text);
  serializer.serializeBool(this.confirmed);
}

static deserialize(deserializer: Deserializer): ViewModel {
  const text = deserializer.deserializeStr();
  const confirmed = deserializer.deserializeBool();
  return new ViewModel(text,confirmed);
}

}
export class Helpers {
  static serializeVectorHttpHeader(value: Seq<HttpHeader>, serializer: Serializer): void {
    serializer.serializeLen(value.length);
    value.forEach((item: HttpHeader) => {
        item.serialize(serializer);
    });
  }

  static deserializeVectorHttpHeader(deserializer: Deserializer): Seq<HttpHeader> {
    const length = deserializer.deserializeLen();
    const list: Seq<HttpHeader> = [];
    for (let i = 0; i < length; i++) {
        list.push(HttpHeader.deserialize(deserializer));
    }
    return list;
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

