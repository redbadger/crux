import { Serializer, Deserializer } from '../serde/mod';
import { uint32, str } from '../serde/mod';
export declare abstract class Effect {
    abstract serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): Effect;
}
export declare class EffectVariantRender extends Effect {
    value: RenderOperation;
    constructor(value: RenderOperation);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): EffectVariantRender;
}
export declare abstract class Event {
    abstract serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): Event;
}
export declare class EventVariantIncrement extends Event {
    constructor();
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): EventVariantIncrement;
}
export declare class EventVariantDecrement extends Event {
    constructor();
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): EventVariantDecrement;
}
export declare class EventVariantReset extends Event {
    constructor();
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): EventVariantReset;
}
export declare class RenderOperation {
    constructor();
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): RenderOperation;
}
export declare class Request {
    id: uint32;
    effect: Effect;
    constructor(id: uint32, effect: Effect);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): Request;
}
export declare class ViewModel {
    count: str;
    constructor(count: str);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): ViewModel;
}
export declare class Helpers {
}
