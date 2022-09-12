import { register, init } from "svelte-i18n";

export enum Language {
	en = "en",
	de = "de",
	fr = "fr",
	pl = "pl",
	ru = "ru"
}

export function asCountryCode(lang: Language) {
	switch (lang) {
		case Language.en:
			return "gb";
		case Language.de:
			return "de";
		case Language.fr:
			return "fr";
		case Language.pl:
			return "pl";
		case Language.ru:
			return "ru";
	}
}

export function language_from_game_language(lang: string): Language {
	switch (lang) {
		case "English":
			return Language.en;
		case "German":
			return Language.de;
		case "French":
			return Language.fr;
		case "Polish":
			return Language.pl;
		case "Russian":
			return Language.ru;
		default:
			return Language.en;
	}
}

export const SUPPORTED_LANGUAGES = [Language.en, Language.de];

export default function loadI18n(initialLang: string) {
	register("en", () => import("./en.json"));
	register("de", () => import("./de.json"));

	init({
		fallbackLocale: "en",
		initialLocale: initialLang
	});
}
