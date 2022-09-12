<script lang="ts">
	import { _ } from "svelte-i18n";
	import { mods as modStore } from "$lib/modStore";

	// SMUI
	import DataTable, { Head, Body, Row, Cell } from "@smui/data-table";
	import IconButton from "@smui/icon-button";
	import Checkbox from "@smui/checkbox";
	import Dialog, { Title, Content } from "@smui/dialog";
	import Card from "@smui/card";

	// Components
	import ErrorDialog from "$lib/ErrorDialog.svelte";
	import { getErrorMessage, invokeBackend, isError } from "./backendErrorHandling";

	$: mods = Array.from($modStore.values());

	let showErrorMessage = false;
	let errorMessage = "";

	let showModInfoDialog = false;
	let modInfoDialogContent = "";
	let modInfoDialogTitle = "";

	function showModInfo(uid: number) {
		const mod = $modStore.get(uid);

		if (!mod) {
			return;
		}

		modInfoDialogContent = mod.info!;
		modInfoDialogTitle = mod.name;
		showModInfoDialog = true;
	}

	async function deleteMod(uid: number) {
		const result = await invokeBackend("delete_mod", { uid });

		if (isError(result)) {
			errorMessage = getErrorMessage(result);
			showErrorMessage = true;
		}
	}

	async function activate_mod(uid: number) {
		const result = await invokeBackend("activate_mod", { uid });

		if (isError(result)) {
			// Reset is_active due to binding
			modStore.update((mods) => {
				const mod = mods.get(uid);

				if (mod) {
					mod.is_active = false;
					mods.set(uid, mod);
				}

				return mods;
			});

			errorMessage = getErrorMessage(result);
			showErrorMessage = true;
		}
	}

	async function deactivate_mod(uid: number) {
		const result = await invokeBackend("deactivate_mod", { uid });

		if (isError(result)) {
			// Reset is_active due to binding
			modStore.update((mods) => {
				const mod = mods.get(uid);

				if (mod) {
					mod.is_active = true;
					mods.set(uid, mod);
				}

				return mods;
			});

			errorMessage = getErrorMessage(result);
			showErrorMessage = true;
		}
	}
</script>

<DataTable table$aria-label="Mod list" id="modTable">
	<Head id="modTableHead">
		<Row>
			<Cell class="modTableHeadCell">{$_("content.modActive")}</Cell>
			<Cell class="modTableHeadCell" style="width: 100%;">{$_("content.modName")}</Cell>
			<Cell class="modTableHeadCell">{$_("content.modVersion")}</Cell>
			<Cell class="modTableHeadCell">{$_("content.modAuthor")}</Cell>
			<Cell class="modTableHeadCell">{$_("content.modActions")}</Cell>
		</Row>
	</Head>
	<Body>
		{#each mods as mod (mod.uid)}
			<Row>
				<Cell checkbox
					><Checkbox
						bind:checked={mod.is_active}
						on:click={() => {
							if (!mod.is_active) {
								activate_mod(mod.uid);
							} else {
								deactivate_mod(mod.uid);
							}
						}}
					/></Cell
				>
				<Cell>{mod.name}</Cell>
				<Cell>{mod.version ? mod.version : "n/a"}</Cell>
				<Cell>{mod.author ? mod.author : "n/a"}</Cell>
				<Cell checkbox>
					<IconButton
						style="float: right;"
						size="mini"
						class="material-icons"
						id="deleteMod"
						aria-label="Delete Mod"
						on:click={() => deleteMod(mod.uid)}>delete</IconButton
					>
					{#if mod.info}
						<IconButton
							style=""
							size="mini"
							class="material-icons"
							id="infoMod"
							aria-label="Mod info"
							on:click={() => showModInfo(mod.uid)}>info</IconButton
						>
					{/if}
				</Cell>
			</Row>
		{/each}
	</Body>
</DataTable>

{#if mods.length === 0}
	<div style="display: flex; justify-content: center;">
		<Card
			style="text-align: center; margin-top: 20px; max-width: 80%; background: none; border: none;"
			variant="outlined"
		>
			<Content style="margin: 5px">{$_("content.noModsFound")}</Content>
		</Card>
	</div>
{/if}

<Dialog bind:open={showModInfoDialog}>
	<Title>{modInfoDialogTitle}</Title>
	<Content style="white-space: pre-wrap;">{modInfoDialogContent}</Content>
</Dialog>

<ErrorDialog bind:open={showErrorMessage} message={errorMessage} />
