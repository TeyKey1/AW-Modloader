<script lang="ts">
	import { open } from "@tauri-apps/api/dialog";
	import { open as openBrowser } from "@tauri-apps/api/shell";
	import { appWindow } from "@tauri-apps/api/window";
	import { _ } from "svelte-i18n";
	import { onDestroy } from "svelte";
	import type { UnlistenFn } from "@tauri-apps/api/event";
	import { getErrorMessage, invokeBackend, isError } from "$lib/backendErrorHandling";

	// SMUI
	import Button, { Icon, Label } from "@smui/button";
	import Dialog, { Title, Content, Actions, Header } from "@smui/dialog";
	import IconButton from "@smui/icon-button";
	import CircularProgress from "@smui/circular-progress";

	// Components
	import ErrorDialog from "$lib/ErrorDialog.svelte";
	import { FORUM_HELP } from "./url";

	let showErrorMessage = false;
	let errorMessage = "";

	let showHelp = false;

	let addModLoading = false;

	async function addMod() {
		addModLoading = true;
		let selectedFiles = await open({
			directory: false,
			multiple: true,
			title: $_("footer.modSelection"),
			filters: [
				{
					name: "Archives",
					extensions: ["7z", "zip"]
				}
			]
		});

		if (!selectedFiles) {
			addModLoading = false;
			return;
		}

		if (!Array.isArray(selectedFiles)) {
			selectedFiles = [selectedFiles];
		}

		const result = await invokeBackend("add_new_mod", { archivePath: selectedFiles[0] });

		if (isError(result)) {
			errorMessage = getErrorMessage(result);
			showErrorMessage = true;
		}

		addModLoading = false;
	}

	let unlistenOverwrite: null | UnlistenFn = null;
	let openOverwriteDialog = false;
	let overwriteDialogModName = "";

	appWindow
		.listen("add-mod-ask-overwrite", (event) => {
			openOverwriteDialog = true;
			overwriteDialogModName = event.payload as string;
		})
		.then((unlisten) => {
			unlistenOverwrite = unlisten;
		});

	function sendOverwriteAnswer(overwrite: boolean) {
		openOverwriteDialog = false;
		appWindow.emit("add-mod-overwrite", { overwrite });
	}

	onDestroy(() => {
		// Deregister event handlers
		if (unlistenOverwrite) {
			unlistenOverwrite();
		}
	});
</script>

<div class="appFooter">
	<Button
		style="float: left; margin-left: 10px"
		class="infoButton"
		on:click={() => {
			showHelp = true;
		}}
	>
		<Icon class="material-icons">help_center</Icon>
		<Label>{$_("footer.help")}</Label>
	</Button>
	<Button
		style="float: right; margin-right: 10px"
		color="primary"
		on:click={addMod}
		disabled={addModLoading}
	>
		{#if !addModLoading}
			<Icon class="material-icons">add</Icon>
			<Label>{$_("footer.addMod")}</Label>
		{:else}
			<CircularProgress style="height: 28px; width: 28px;" indeterminate />
		{/if}
	</Button>

	<ErrorDialog bind:open={showErrorMessage} message={errorMessage} />

	<Dialog bind:open={openOverwriteDialog} scrimClickAction="" escapeKeyAction="">
		<Title>{$_("content.overwriteModTitle")}</Title>
		<Content>
			{$_("content.overwriteMod", { values: { modName: overwriteDialogModName } })}
		</Content>
		<Actions>
			<Button class="errorButton" on:click={() => sendOverwriteAnswer(false)}>
				<Label>{$_("ui.cancel")}</Label>
			</Button>
			<Button class="successButton" on:click={() => sendOverwriteAnswer(true)}>
				<Label>{$_("ui.overwrite")}</Label>
			</Button>
		</Actions>
	</Dialog>

	<Dialog
		bind:open={showHelp}
		fullscreen
		aria-labelledby="fullscreen-title"
		aria-describedby="fullscreen-content"
		on:SMUIDialog:closed={() => (showHelp = false)}
	>
		<Header>
			<Title>{$_("footer.help")}</Title>
			<IconButton action="close" class="material-icons">close</IconButton>
		</Header>
		<Content style="white-space: pre-wrap;"><span>{@html $_("footer.helpText")}</span></Content>
		<Actions>
			<Button class="infoButton" on:click={() => openBrowser(FORUM_HELP)}>
				<Label>Ask in Forum</Label>
			</Button>
		</Actions>
	</Dialog>
</div>
