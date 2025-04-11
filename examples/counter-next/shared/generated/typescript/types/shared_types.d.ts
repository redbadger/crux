import { Serializer, Deserializer } from '../serde/mod';
import { Seq, bool, uint8, uint16, uint32, str, bytes } from '../serde/mod';
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
export declare class EffectVariantHttp extends Effect {
    value: HttpRequest;
    constructor(value: HttpRequest);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): EffectVariantHttp;
}
export declare class EffectVariantServerSentEvents extends Effect {
    value: SseRequest;
    constructor(value: SseRequest);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): EffectVariantServerSentEvents;
}
export declare abstract class Event {
    abstract serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): Event;
}
export declare class EventVariantGet extends Event {
    constructor();
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): EventVariantGet;
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
export declare class EventVariantStartWatch extends Event {
    constructor();
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): EventVariantStartWatch;
}
export declare abstract class HttpError {
    abstract serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): HttpError;
}
export declare class HttpErrorVariantUrl extends HttpError {
    value: str;
    constructor(value: str);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): HttpErrorVariantUrl;
}
export declare class HttpErrorVariantIo extends HttpError {
    value: str;
    constructor(value: str);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): HttpErrorVariantIo;
}
export declare class HttpErrorVariantTimeout extends HttpError {
    constructor();
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): HttpErrorVariantTimeout;
}
export declare class HttpHeader {
    name: str;
    value: str;
    constructor(name: str, value: str);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): HttpHeader;
}
export declare class HttpRequest {
    method: str;
    url: str;
    headers: Seq<HttpHeader>;
    body: bytes;
    constructor(method: str, url: str, headers: Seq<HttpHeader>, body: bytes);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): HttpRequest;
}
export declare class HttpResponse {
    status: uint16;
    headers: Seq<HttpHeader>;
    body: bytes;
    constructor(status: uint16, headers: Seq<HttpHeader>, body: bytes);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): HttpResponse;
}
export declare abstract class HttpResult {
    abstract serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): HttpResult;
}
export declare class HttpResultVariantOk extends HttpResult {
    value: HttpResponse;
    constructor(value: HttpResponse);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): HttpResultVariantOk;
}
export declare class HttpResultVariantErr extends HttpResult {
    value: HttpError;
    constructor(value: HttpError);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): HttpResultVariantErr;
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
export declare class SseRequest {
    url: str;
    constructor(url: str);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): SseRequest;
}
export declare abstract class SseResponse {
    abstract serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): SseResponse;
}
export declare class SseResponseVariantChunk extends SseResponse {
    value: Seq<uint8>;
    constructor(value: Seq<uint8>);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): SseResponseVariantChunk;
}
export declare class SseResponseVariantDone extends SseResponse {
    constructor();
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): SseResponseVariantDone;
}
export declare class ViewModel {
    text: str;
    confirmed: bool;
    constructor(text: str, confirmed: bool);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): ViewModel;
}
export declare class Helpers {
    static serializeVectorHttpHeader(value: Seq<HttpHeader>, serializer: Serializer): void;
    static deserializeVectorHttpHeader(deserializer: Deserializer): Seq<HttpHeader>;
    static serializeVectorU8(value: Seq<uint8>, serializer: Serializer): void;
    static deserializeVectorU8(deserializer: Deserializer): Seq<uint8>;
}
