import { process_event, view } from 'shared';
import initCore from 'shared';
import { writable } from 'svelte/store';
import { EffectVariantRender, ViewModel, Request } from 'shared_types/types/shared_types';
import type { Effect, Event } from 'shared_types/types/shared_types';
import { BincodeSerializer, BincodeDeserializer } from 'shared_types/bincode/mod';

const { subscribe, set } = writable(new ViewModel('0'));

export async function update(event: Event) {
	console.log('event', event);
	await initCore();

	const serializer = new BincodeSerializer();
	event.serialize(serializer);

	const effects = process_event(serializer.getBytes());
	const requests = deserializeRequests(effects);
	for (const { id, effect } of requests) {
		processEffect(id, effect);
	}
}

function processEffect(_id: number, effect: Effect) {
	console.log('effect', effect);
	switch (effect.constructor) {
		case EffectVariantRender: {
			set(deserializeView(view()));
			break;
		}
	}
}

function deserializeRequests(bytes: Uint8Array): Request[] {
	const deserializer = new BincodeDeserializer(bytes);
	const len = deserializer.deserializeLen();
	const requests: Request[] = [];
	for (let i = 0; i < len; i++) {
		const request = Request.deserialize(deserializer);
		requests.push(request);
	}
	return requests;
}

function deserializeView(bytes: Uint8Array): ViewModel {
	return ViewModel.deserialize(new BincodeDeserializer(bytes));
}

export default {
	subscribe
};
