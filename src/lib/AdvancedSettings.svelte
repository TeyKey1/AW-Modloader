<script lang="ts">
	import { _, locale } from "svelte-i18n";
	import { Language, asCountryCode, language_from_game_language } from "../locale/i18n";
	import { open as openFileDialog } from "@tauri-apps/api/dialog";
	import { getErrorMessage, invokeBackend, isError } from "$lib/backendErrorHandling";

	// SMUI
	import Dialog, { Title, Content, Actions } from "@smui/dialog";
	import Button, { Label } from "@smui/button";
	import Select, { Option } from "@smui/select";
	import Textfield from "@smui/textfield";
	import Icon from "@smui/textfield/icon";
	import { onMount } from "svelte";

	export let open = false;
	export let initialConfig = false;

	let selectedGameLanguage = (() => {
		switch ($locale) {
			case "de":
				return Language.de;
			case "en":
				return Language.en;
			default:
				return Language.en;
		}
	})();

	let selectedGamePath = "";

	async function selectGamePath() {
		let selectedFolder = await openFileDialog({
			directory: true,
			multiple: false,
			title: $_("settings.initialConfig.configDialogGamePath"),
			filters: []
		});

		if (!selectedFolder) {
			return;
		}

		if (Array.isArray(selectedFolder)) {
			selectedFolder = selectedFolder[0];
		}

		selectedGamePath = selectedFolder;
	}

	let showConfigurationError = false;
	let configurationErrorMessage = "";

	async function applySettings(e: CustomEvent<any>) {
		// Prevent dialog from closing due to click
		e.stopPropagation();
		showConfigurationError = false;

		const result = await invokeBackend<void>("set_advanced_config", {
			gameLang: selectedGameLanguage,
			gamePath: selectedGamePath
		});

		if (isError(result)) {
			configurationErrorMessage = getErrorMessage(result);
			showConfigurationError = true;
			return;
		}

		open = false;
	}

	$: if (open) {
		showConfigurationError = false;
		fetchInitialData();
	}

	async function fetchInitialData() {
		if (!initialConfig) {
			const result = await invokeBackend<[string | null, string | null]>("get_advanced_config");

			if (!result || isError(result)) {
				return;
			}

			if (result[0]) {
				selectedGameLanguage = language_from_game_language(result[0]);
			}

			if (result[1]) {
				selectedGamePath = result[1];
			}
		}
	}

	onMount(async () => {
		await fetchInitialData();
	});
</script>

<Dialog
	bind:open
	scrimClickAction={initialConfig ? "" : "close"}
	escapeKeyAction={initialConfig ? "" : "close"}
	id="initialConfigurationDialog"
>
	<Title
		>{$_(`settings.${initialConfig ? "initialConfig" : "advancedConfig"}.configDialogTitle`)}</Title
	>
	<Content style="overflow: visible;">
		{$_(`settings.${initialConfig ? "initialConfig" : "advancedConfig"}.configDialogExplanation`)}

		<Select
			bind:value={selectedGameLanguage}
			label={$_("settings.initialConfig.configDialogGameLanguage")}
			style="width: 100%;"
		>
			<span
				slot="leadingIcon"
				class={`fi fi-${asCountryCode(selectedGameLanguage)}`}
				style="margin-right: 10px"
			/>
			{#each Object.values(Language) as lang}
				<Option value={lang}
					><span class={`fi fi-${asCountryCode(lang)}`} style="margin-right: 10px" />{$_(
						`settings.language.languages.${lang}`
					)}</Option
				>
			{/each}
		</Select>

		<i>{$_("settings.initialConfig.configDialogGameLanguageHint")}</i>

		<Textfield
			disabled
			bind:value={selectedGamePath}
			label={$_("settings.initialConfig.configDialogGamePath")}
			style="width: 100%;"
			class="fileInputTextField"
			on:click={selectGamePath}
		>
			<Icon class="material-icons" slot="trailingIcon">folder</Icon>
		</Textfield>
		<i>{$_("settings.initialConfig.configDialogGamePathHint")}</i>
		<br />
		{#if showConfigurationError}
			<p style="white-space: pre-wrap;" class="errorButton">
				{$_("ui.error")}: {configurationErrorMessage}
			</p>
		{/if}
	</Content>
	<Actions>
		{#if !initialConfig}
			<Button class="errorButton" on:click={() => (open = false)}>
				<Label>{$_("ui.cancel")}</Label>
			</Button>
		{/if}
		<Button class="successButton" on:click={applySettings}>
			<Label>{$_("settings.initialConfig.configDialogSave")}</Label>
		</Button>
	</Actions>
</Dialog>
