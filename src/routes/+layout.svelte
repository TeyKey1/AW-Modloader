<script lang="ts">
	import { invoke } from "@tauri-apps/api";
	import { onMount } from "svelte";
	import { isLoading, locale } from "svelte-i18n";

	// SMUI
	import CircularProgress from "@smui/circular-progress";

	// Components
	import AppBar from "$lib/AppBar.svelte";
	import AppFooter from "$lib/AppFooter.svelte";

	onMount(async () => {
		let lang: string | null = await invoke("get_app_language");

		if (!lang) {
			lang = "en";
		}
		locale.set(lang);
	});
</script>

{#if !$isLoading}
	{#await onMount then _}
		<AppBar />
		<div style="overflow: auto; height: 100%">
			<slot />
		</div>
		<AppFooter />
	{/await}
{:else}
	<div style="overflow: hidden; height: 100%">
		<div
			style="display: flex; justify-content: center; align-items: center; height: 100%; flex-direction: column;"
		>
			<CircularProgress style="height: 128px; width: 128px;" indeterminate />
			<strong style="margin-top: 20px">Loading...</strong>
		</div>
	</div>
{/if}
