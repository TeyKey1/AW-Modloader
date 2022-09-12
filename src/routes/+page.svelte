<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/tauri";

	// SMUI
	import type { SnackbarComponentDev } from "@smui/snackbar";

	// CSS
	import "/node_modules/flag-icons/css/flag-icons.min.css";

	// Components
	import ModList from "$lib/ModList.svelte";
	import ErrorSnackbar from "$lib/ErrorSnackbar.svelte";
	import InitialConfigurationDialog from "$lib/InitialConfigDialog.svelte";

	let errorSnackbar: SnackbarComponentDev;
	let errorMessage = "";

	function displayError(e: PromiseRejectionEvent) {
		errorMessage = e.reason;
		errorSnackbar.open();
	}

	let showInitialConfigDialog = false;

	onMount(async () => {
		// Catch all uncaught errors and display them
		window.onunhandledrejection = (e) => {
			displayError(e);
			return false;
		};

		// Check if the user needs to supply initial configuration data (This usually happens on first start of the app after installation)
		let config_initialized: boolean = await invoke("config_is_initialized");

		if (!config_initialized) {
			showInitialConfigDialog = true;
		}
	});
</script>

<ModList />

<InitialConfigurationDialog bind:open={showInitialConfigDialog} />

<ErrorSnackbar bind:errorSnackbar message={errorMessage} timeoutMs={7000} />
