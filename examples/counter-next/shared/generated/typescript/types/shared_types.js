"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Helpers = exports.ViewModel = exports.SseResponseVariantDone = exports.SseResponseVariantChunk = exports.SseResponse = exports.SseRequest = exports.Request = exports.RenderOperation = exports.HttpResultVariantErr = exports.HttpResultVariantOk = exports.HttpResult = exports.HttpResponse = exports.HttpRequest = exports.HttpHeader = exports.HttpErrorVariantTimeout = exports.HttpErrorVariantIo = exports.HttpErrorVariantUrl = exports.HttpError = exports.EventVariantStartWatch = exports.EventVariantDecrement = exports.EventVariantIncrement = exports.EventVariantGet = exports.Event = exports.EffectVariantServerSentEvents = exports.EffectVariantHttp = exports.EffectVariantRender = exports.Effect = void 0;
class Effect {
    static deserialize(deserializer) {
        const index = deserializer.deserializeVariantIndex();
        switch (index) {
            case 0: return EffectVariantRender.load(deserializer);
            case 1: return EffectVariantHttp.load(deserializer);
            case 2: return EffectVariantServerSentEvents.load(deserializer);
            default: throw new Error("Unknown variant index for Effect: " + index);
        }
    }
}
exports.Effect = Effect;
class EffectVariantRender extends Effect {
    constructor(value) {
        super();
        this.value = value;
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(0);
        this.value.serialize(serializer);
    }
    static load(deserializer) {
        const value = RenderOperation.deserialize(deserializer);
        return new EffectVariantRender(value);
    }
}
exports.EffectVariantRender = EffectVariantRender;
class EffectVariantHttp extends Effect {
    constructor(value) {
        super();
        this.value = value;
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(1);
        this.value.serialize(serializer);
    }
    static load(deserializer) {
        const value = HttpRequest.deserialize(deserializer);
        return new EffectVariantHttp(value);
    }
}
exports.EffectVariantHttp = EffectVariantHttp;
class EffectVariantServerSentEvents extends Effect {
    constructor(value) {
        super();
        this.value = value;
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(2);
        this.value.serialize(serializer);
    }
    static load(deserializer) {
        const value = SseRequest.deserialize(deserializer);
        return new EffectVariantServerSentEvents(value);
    }
}
exports.EffectVariantServerSentEvents = EffectVariantServerSentEvents;
class Event {
    static deserialize(deserializer) {
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
exports.Event = Event;
class EventVariantGet extends Event {
    constructor() {
        super();
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(0);
    }
    static load(deserializer) {
        return new EventVariantGet();
    }
}
exports.EventVariantGet = EventVariantGet;
class EventVariantIncrement extends Event {
    constructor() {
        super();
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(1);
    }
    static load(deserializer) {
        return new EventVariantIncrement();
    }
}
exports.EventVariantIncrement = EventVariantIncrement;
class EventVariantDecrement extends Event {
    constructor() {
        super();
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(2);
    }
    static load(deserializer) {
        return new EventVariantDecrement();
    }
}
exports.EventVariantDecrement = EventVariantDecrement;
class EventVariantStartWatch extends Event {
    constructor() {
        super();
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(3);
    }
    static load(deserializer) {
        return new EventVariantStartWatch();
    }
}
exports.EventVariantStartWatch = EventVariantStartWatch;
class HttpError {
    static deserialize(deserializer) {
        const index = deserializer.deserializeVariantIndex();
        switch (index) {
            case 0: return HttpErrorVariantUrl.load(deserializer);
            case 1: return HttpErrorVariantIo.load(deserializer);
            case 2: return HttpErrorVariantTimeout.load(deserializer);
            default: throw new Error("Unknown variant index for HttpError: " + index);
        }
    }
}
exports.HttpError = HttpError;
class HttpErrorVariantUrl extends HttpError {
    constructor(value) {
        super();
        this.value = value;
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(0);
        serializer.serializeStr(this.value);
    }
    static load(deserializer) {
        const value = deserializer.deserializeStr();
        return new HttpErrorVariantUrl(value);
    }
}
exports.HttpErrorVariantUrl = HttpErrorVariantUrl;
class HttpErrorVariantIo extends HttpError {
    constructor(value) {
        super();
        this.value = value;
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(1);
        serializer.serializeStr(this.value);
    }
    static load(deserializer) {
        const value = deserializer.deserializeStr();
        return new HttpErrorVariantIo(value);
    }
}
exports.HttpErrorVariantIo = HttpErrorVariantIo;
class HttpErrorVariantTimeout extends HttpError {
    constructor() {
        super();
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(2);
    }
    static load(deserializer) {
        return new HttpErrorVariantTimeout();
    }
}
exports.HttpErrorVariantTimeout = HttpErrorVariantTimeout;
class HttpHeader {
    constructor(name, value) {
        this.name = name;
        this.value = value;
    }
    serialize(serializer) {
        serializer.serializeStr(this.name);
        serializer.serializeStr(this.value);
    }
    static deserialize(deserializer) {
        const name = deserializer.deserializeStr();
        const value = deserializer.deserializeStr();
        return new HttpHeader(name, value);
    }
}
exports.HttpHeader = HttpHeader;
class HttpRequest {
    constructor(method, url, headers, body) {
        this.method = method;
        this.url = url;
        this.headers = headers;
        this.body = body;
    }
    serialize(serializer) {
        serializer.serializeStr(this.method);
        serializer.serializeStr(this.url);
        Helpers.serializeVectorHttpHeader(this.headers, serializer);
        serializer.serializeBytes(this.body);
    }
    static deserialize(deserializer) {
        const method = deserializer.deserializeStr();
        const url = deserializer.deserializeStr();
        const headers = Helpers.deserializeVectorHttpHeader(deserializer);
        const body = deserializer.deserializeBytes();
        return new HttpRequest(method, url, headers, body);
    }
}
exports.HttpRequest = HttpRequest;
class HttpResponse {
    constructor(status, headers, body) {
        this.status = status;
        this.headers = headers;
        this.body = body;
    }
    serialize(serializer) {
        serializer.serializeU16(this.status);
        Helpers.serializeVectorHttpHeader(this.headers, serializer);
        serializer.serializeBytes(this.body);
    }
    static deserialize(deserializer) {
        const status = deserializer.deserializeU16();
        const headers = Helpers.deserializeVectorHttpHeader(deserializer);
        const body = deserializer.deserializeBytes();
        return new HttpResponse(status, headers, body);
    }
}
exports.HttpResponse = HttpResponse;
class HttpResult {
    static deserialize(deserializer) {
        const index = deserializer.deserializeVariantIndex();
        switch (index) {
            case 0: return HttpResultVariantOk.load(deserializer);
            case 1: return HttpResultVariantErr.load(deserializer);
            default: throw new Error("Unknown variant index for HttpResult: " + index);
        }
    }
}
exports.HttpResult = HttpResult;
class HttpResultVariantOk extends HttpResult {
    constructor(value) {
        super();
        this.value = value;
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(0);
        this.value.serialize(serializer);
    }
    static load(deserializer) {
        const value = HttpResponse.deserialize(deserializer);
        return new HttpResultVariantOk(value);
    }
}
exports.HttpResultVariantOk = HttpResultVariantOk;
class HttpResultVariantErr extends HttpResult {
    constructor(value) {
        super();
        this.value = value;
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(1);
        this.value.serialize(serializer);
    }
    static load(deserializer) {
        const value = HttpError.deserialize(deserializer);
        return new HttpResultVariantErr(value);
    }
}
exports.HttpResultVariantErr = HttpResultVariantErr;
class RenderOperation {
    constructor() {
    }
    serialize(serializer) {
    }
    static deserialize(deserializer) {
        return new RenderOperation();
    }
}
exports.RenderOperation = RenderOperation;
class Request {
    constructor(id, effect) {
        this.id = id;
        this.effect = effect;
    }
    serialize(serializer) {
        serializer.serializeU32(this.id);
        this.effect.serialize(serializer);
    }
    static deserialize(deserializer) {
        const id = deserializer.deserializeU32();
        const effect = Effect.deserialize(deserializer);
        return new Request(id, effect);
    }
}
exports.Request = Request;
class SseRequest {
    constructor(url) {
        this.url = url;
    }
    serialize(serializer) {
        serializer.serializeStr(this.url);
    }
    static deserialize(deserializer) {
        const url = deserializer.deserializeStr();
        return new SseRequest(url);
    }
}
exports.SseRequest = SseRequest;
class SseResponse {
    static deserialize(deserializer) {
        const index = deserializer.deserializeVariantIndex();
        switch (index) {
            case 0: return SseResponseVariantChunk.load(deserializer);
            case 1: return SseResponseVariantDone.load(deserializer);
            default: throw new Error("Unknown variant index for SseResponse: " + index);
        }
    }
}
exports.SseResponse = SseResponse;
class SseResponseVariantChunk extends SseResponse {
    constructor(value) {
        super();
        this.value = value;
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(0);
        Helpers.serializeVectorU8(this.value, serializer);
    }
    static load(deserializer) {
        const value = Helpers.deserializeVectorU8(deserializer);
        return new SseResponseVariantChunk(value);
    }
}
exports.SseResponseVariantChunk = SseResponseVariantChunk;
class SseResponseVariantDone extends SseResponse {
    constructor() {
        super();
    }
    serialize(serializer) {
        serializer.serializeVariantIndex(1);
    }
    static load(deserializer) {
        return new SseResponseVariantDone();
    }
}
exports.SseResponseVariantDone = SseResponseVariantDone;
class ViewModel {
    constructor(text, confirmed) {
        this.text = text;
        this.confirmed = confirmed;
    }
    serialize(serializer) {
        serializer.serializeStr(this.text);
        serializer.serializeBool(this.confirmed);
    }
    static deserialize(deserializer) {
        const text = deserializer.deserializeStr();
        const confirmed = deserializer.deserializeBool();
        return new ViewModel(text, confirmed);
    }
}
exports.ViewModel = ViewModel;
class Helpers {
    static serializeVectorHttpHeader(value, serializer) {
        serializer.serializeLen(value.length);
        value.forEach((item) => {
            item.serialize(serializer);
        });
    }
    static deserializeVectorHttpHeader(deserializer) {
        const length = deserializer.deserializeLen();
        const list = [];
        for (let i = 0; i < length; i++) {
            list.push(HttpHeader.deserialize(deserializer));
        }
        return list;
    }
    static serializeVectorU8(value, serializer) {
        serializer.serializeLen(value.length);
        value.forEach((item) => {
            serializer.serializeU8(item);
        });
    }
    static deserializeVectorU8(deserializer) {
        const length = deserializer.deserializeLen();
        const list = [];
        for (let i = 0; i < length; i++) {
            list.push(deserializer.deserializeU8());
        }
        return list;
    }
}
exports.Helpers = Helpers;
