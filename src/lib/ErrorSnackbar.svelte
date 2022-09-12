<script lang="ts">
	/*
	 * Error Snackbar which is used to display errors fast without the need of user confirmation
	 */
	import { _ } from "svelte-i18n";

	// SMUI
	import Snackbar, { Label, type SnackbarComponentDev } from "@smui/snackbar";

	export let errorSnackbar: SnackbarComponentDev;
	export let message: string;
	export let timeoutMs: number = 4000;
</script>

<Snackbar bind:this={errorSnackbar} {timeoutMs} class="errorSnackbar">
	<Label><strong>{$_("ui.error")}</strong>: {message}</Label>
</Snackbar>
