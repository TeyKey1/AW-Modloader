<script lang="ts">
	import { open } from "@tauri-apps/api/shell";

	// SMUI
	import Dialog, { Header, Title, Content, Actions } from "@smui/dialog";
	import IconButton from "@smui/icon-button";
	import Button from "@smui/button";
	import { SOURCE_CODE } from "./url";

	export let showAbout = false;
</script>

<Dialog
	bind:open={showAbout}
	fullscreen
	aria-labelledby="fullscreen-title"
	aria-describedby="fullscreen-content"
	on:SMUIDialog:closed={() => (showAbout = false)}
>
	<Header>
		<Title>About</Title>
		<IconButton action="close" class="material-icons">close</IconButton>
	</Header>
	<Content>
		<p>
			This Modloader was created as a small side project to learn about svelte and the
			tauri-framework. <br />
			<br /><strong>Creator:</strong> TeyKey1 (github.com/TeyKey1) and Contributors
		</p>
	</Content>
	<Actions>
		<Button on:click={() => open(SOURCE_CODE)}>Source on Github</Button>
	</Actions>
</Dialog>
