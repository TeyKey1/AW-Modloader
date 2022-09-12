<script lang="ts">
	/*
	 * Error Dialog which is used to display errors that require user confirmation
	 */

	// SMUI
	import Dialog, { Title, Content, Actions } from "@smui/dialog";
	import Button, { Label } from "@smui/button";

	export let open = false;
	export let message: string;
</script>

<Dialog bind:open>
	<!-- Title cannot contain leading whitespace due to mdc-typography-baseline-top() -->
	<Title class="errorDialogTitle">Error</Title>
	<Content style="white-space: pre-wrap;">{message}</Content>
	<Actions class="actionsCentered">
		<Button on:click={() => (open = false)}>
			<Label>Ok</Label>
		</Button>
	</Actions>
</Dialog>
