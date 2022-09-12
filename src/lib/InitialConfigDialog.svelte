<script lang="ts">
	import { _, locale } from "svelte-i18n";
	import { Language, asCountryCode, SUPPORTED_LANGUAGES } from "../locale/i18n";
	import { invokeBackend } from "$lib/backendErrorHandling";

	// SMUI
	import Dialog, { Title, Content, Actions, InitialFocus } from "@smui/dialog";
	import Button, { Label } from "@smui/button";
	import List, { Item, Graphic, Text } from "@smui/list";
	import Radio from "@smui/radio";

	// Components
	import AdvancedSettings from "$lib/AdvancedSettings.svelte";

	export let open = false;

	let openConfigDialog = false;

	let selectedAppLanguage = Language.en;

	async function selectAppLanguage() {
		open = false;

		await invokeBackend("set_app_language", { lang: selectedAppLanguage });

		openConfigDialog = true;
	}

	$: locale.set(selectedAppLanguage);
</script>

<Dialog bind:open scrimClickAction="" escapeKeyAction="">
	<Title>{$_("settings.initialConfig.languageDialogTitle")}</Title>
	<Content>
		<List radioList>
			{#each SUPPORTED_LANGUAGES as lang}
				<Item use={[InitialFocus]}>
					<Graphic>
						<Radio bind:group={selectedAppLanguage} value={lang} />
					</Graphic>
					<Text
						><span class={`fi fi-${asCountryCode(lang)}`} style="margin-right: 10px" />
						{$_(`settings.language.languages.${lang}`)}</Text
					>
				</Item>
			{/each}
		</List>
	</Content>
	<Actions>
		<Button class="successButton" on:click={selectAppLanguage}>
			<Label>{$_("settings.initialConfig.languageDialogSelect")}</Label>
		</Button>
	</Actions>
</Dialog>

<AdvancedSettings bind:open={openConfigDialog} initialConfig={true} />
