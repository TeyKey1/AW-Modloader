<script lang="ts">
	import { onMount } from "svelte";
	import { _, locale } from "svelte-i18n";
	import { SUPPORTED_LANGUAGES, Language, asCountryCode } from "../locale/i18n";
	import { invokeBackend } from "./backendErrorHandling";

	// SMUI
	import TopAppBar, { Row, Section, Title } from "@smui/top-app-bar";
	import IconButton from "@smui/icon-button";
	import Menu, { type MenuComponentDev } from "@smui/menu";
	import Dialog, { Header, Content } from "@smui/dialog";
	import List, { Item, Text, Separator, Graphic } from "@smui/list";

	// Components
	import About from "$lib/About.svelte";
	import AdvancedSettings from "$lib/AdvancedSettings.svelte";

	let settings: MenuComponentDev;
	let settingsAnchor: HTMLDivElement;

	let darkTheme: boolean;

	$: themeIcon = darkTheme ? "light_mode" : "dark_mode";
	$: themeText = darkTheme ? $_("settings.menu.lightTheme") : $_("settings.menu.darkTheme");

	let showAbout = false;
	let showLanguageSelect = false;

	let showAdvancedSettings = false;

	onMount(async () => {
		darkTheme = (await invokeBackend("get_dark_mode")) as boolean;
	});

	async function changeTheme() {
		darkTheme = !darkTheme;

		await invokeBackend("configure_dark_mode", { dark: darkTheme });
	}

	async function selectAppLanguage(lang: Language) {
		locale.set(lang);
		await invokeBackend("set_app_language", { lang });
		showLanguageSelect = false;
	}
</script>

<svelte:head>
	{#if darkTheme === undefined}
		<link rel="stylesheet" href="/smui.css" media="(prefers-color-scheme: light)" />
		<link rel="stylesheet" href="/smui-dark.css" media="screen and (prefers-color-scheme: dark)" />
	{:else if darkTheme}
		<link rel="stylesheet" href="/smui.css" media="print" />
		<link rel="stylesheet" href="/smui-dark.css" media="screen" />
	{:else}
		<link rel="stylesheet" href="/smui.css" />
	{/if}
</svelte:head>

<TopAppBar variant="static" dense id="appBar">
	<Row>
		<Section>
			<Title><strong>{$_("appName")}</strong></Title>
		</Section>
		<Section align="end" toolbar>
			<div bind:this={settingsAnchor}>
				<IconButton
					style="margin-right: 5px"
					class="material-icons appBarButton"
					aria-label="Settings"
					on:click={() => settings.setOpen(true)}
					>settings
				</IconButton>
			</div>
			<Menu
				bind:this={settings}
				anchor={false}
				bind:anchorElement={settingsAnchor}
				anchorCorner="BOTTOM_LEFT"
			>
				<List dense>
					<Item on:click={() => (showAdvancedSettings = true)}>
						<Graphic class="material-icons">construction</Graphic>
						<Text>{$_("settings.menu.advanced")}</Text>
					</Item>
					<Item on:click={() => (showLanguageSelect = true)}>
						<Graphic class="material-icons">translate</Graphic>
						<Text>{$_("settings.menu.language")}</Text>
					</Item>
					<Item on:click={changeTheme}>
						<Graphic class="material-icons">{themeIcon}</Graphic>
						<Text>{themeText}</Text>
					</Item>
					<Separator />
					<Item on:click={() => (showAbout = true)}>
						<Text>{$_("settings.menu.about")}</Text>
					</Item>
				</List>
			</Menu>
		</Section>
	</Row>
</TopAppBar>

<AdvancedSettings bind:open={showAdvancedSettings} initialConfig={false} />

<About bind:showAbout />

<Dialog bind:open={showLanguageSelect} on:SMUIDialog:closed={() => (showLanguageSelect = false)}>
	<Header>
		<Title>{$_("settings.language.chooseLanguage")}</Title>
	</Header>
	<Content>
		<List dense>
			{#each SUPPORTED_LANGUAGES as lang}
				<Item selected={lang === $locale} on:click={() => selectAppLanguage(lang)}
					><span class={`fi fi-${asCountryCode(lang)}`} style="margin-right: 10px" />{$_(
						`settings.language.languages.${lang}`
					)}</Item
				>
			{/each}
		</List>
	</Content>
</Dialog>
