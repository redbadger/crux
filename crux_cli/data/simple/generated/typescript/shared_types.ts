
import { Serializer, Deserializer } from '../serde/mod';

import { Optional, Seq, Tuple, ListTuple, unit, bool, int8, int16, int32, int64, int128, uint8, uint16, uint32, uint64, uint128, float32, float64, char, str, bytes } from '../serde/mod';

export abstract class Effect {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): Effect {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return EffectVariantRender.load(deserializer);
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
export abstract class Event {
abstract serialize(serializer: Serializer): void;

static deserialize(deserializer: Deserializer): Event {
  const index = deserializer.deserializeVariantIndex();
  switch (index) {
    case 0: return EventVariantIncrement.load(deserializer);
    case 1: return EventVariantDecrement.load(deserializer);
    case 2: return EventVariantReset.load(deserializer);
    default: throw new Error("Unknown variant index for Event: " + index);
  }
}
}


export class EventVariantIncrement extends Event {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(0);
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
  serializer.serializeVariantIndex(1);
}

static load(deserializer: Deserializer): EventVariantDecrement {
  return new EventVariantDecrement();
}

}

export class EventVariantReset extends Event {
constructor () {
  super();
}

public serialize(serializer: Serializer): void {
  serializer.serializeVariantIndex(2);
}

static load(deserializer: Deserializer): EventVariantReset {
  return new EventVariantReset();
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
export class ViewModel {

constructor (public count: str) {
}

public serialize(serializer: Serializer): void {
  serializer.serializeStr(this.count);
}

static deserialize(deserializer: Deserializer): ViewModel {
  const count = deserializer.deserializeStr();
  return new ViewModel(count);
}

}
export class Helpers {
}

