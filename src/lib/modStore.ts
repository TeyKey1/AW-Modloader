import { writable, type Writable } from "svelte/store";
import { appWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api";
import type { UnlistenFn } from "@tauri-apps/api/event";

let unlisten: UnlistenFn | null = null;

type ModTreeDataChangedPayload = {
	InsertUpdate?: [number, Mod];
	Delete?: number;
};

type Mod = {
	author: null | string;
	info: null | string;
	injection: string;
	is_active: boolean;
	name: string;
	uid: number;
	version: null | string;
};

export const mods: Writable<Map<number, Mod>> = writable(new Map(), (set) => {
	invoke("get_initial_mod_data").then((data) => {
		const entries: Array<Mod> = Object.values(data as Object);
		let keys = Object.keys(data as Object).map((stringKey) => parseInt(stringKey));
		set(new Map(keys.map((key, idx) => [key, entries[idx]])));

		appWindow
			.listen("mod-tree-data-changed", (event) => {
				const payload = event.payload as ModTreeDataChangedPayload;

				if (payload.InsertUpdate) {
					mods.update((map) => map.set(payload.InsertUpdate![0], payload.InsertUpdate![1]));
				} else if (payload.Delete) {
					mods.update((map) => {
						map.delete(payload.Delete!);
						return map;
					});
				}
			})
			.then((unlistener) => {
				unlisten = unlistener;
			});
	});

	return () => {
		if (unlisten) {
			unlisten();
		}
	};
});
