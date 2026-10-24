<script lang="ts">
	import Callout from '$lib/components/Callout.svelte';
	import Eap8021xFields from '$lib/components/Eap8021xFields.svelte';
	import NetworkApplyStatus from '$lib/components/NetworkApplyStatus.svelte';
	import StepLayout from '$lib/components/StepLayout.svelte';
	import { eap8021xSettings, emptyEap8021xForm } from '$lib/eap-8021x';
	import { t } from '$lib/i18n.svelte';
	import {
		emptyIpForm,
		interfaceType,
		ipSettings,
		type IpForm,
		type Ipv4Method,
		type Ipv6Method,
		type NetworkConfigureRequest
	} from '$lib/network-config';
	import type {
		NetworkApplyState,
		NetworkCheck,
		NetworkInterface,
		WifiNetwork,
		WifiSecurityProfile
	} from '$lib/oobe-state';
	let check = $state<NetworkCheck | null>(null);
	let checking = $state(false);
	let checkError = $state('');
	let selectedInterface = $state('');
	let configure = $state(false);
	let ipv4 = $state(emptyIpForm<Ipv4Method>('auto'));
	let ipv6 = $state(emptyIpForm<Ipv6Method>('auto'));
	let wifiNetworks = $state<WifiNetwork[]>([]);
	let wifiLoading = $state(false);
	let wifiSsid = $state('');
	let wifiSecurity = $state<WifiSecurityProfile['type']>('wpa-personal');
	let wifiPassword = $state('');
	let wifiEnterprise = $state(emptyEap8021xForm());
	let wired8021x = $state(false);
	let wiredSettings = $state(emptyEap8021xForm());
	let hiddenSsid = $state(false);
	let hiddenSsidName = $state('');
	let wifiError = $state('');
	let {
		onBack,
		onContinue,
		interfaces = $bindable<NetworkInterface[]>([]),
		networkConfig = $bindable<NetworkConfigureRequest | null>(null),
		apply = null,
		onWifiScan,
		onWifiConnect,
		onCheck
	}: {
		onBack: () => void;
		onContinue: () => void;
		interfaces?: NetworkInterface[];
		/** The wired configuration to apply on Continue, or null to keep the current one. */
		networkConfig?: NetworkConfigureRequest | null;
		/** A network change being applied. */
		apply?: NetworkApplyState | null;
		onWifiScan?: (interfaceName: string) => Promise<WifiNetwork[]>;
		onWifiConnect?: (
			interfaceName: string,
			ssid: string,
			hidden: boolean,
			security: WifiSecurityProfile
		) => Promise<void>;
		onCheck?: () => Promise<NetworkCheck>;
	} = $props();

	const ipv4Options: { value: Ipv4Method; label: string; description: string }[] = [
		{ value: 'auto', label: 'network-ipv4-auto', description: 'network-ipv4-auto-description' },
		{ value: 'manual', label: 'network-manual', description: 'network-manual-description' },
		{ value: 'disabled', label: 'network-off', description: 'network-ipv4-off-description' }
	];
	const ipv6Options: { value: Ipv6Method; label: string; description: string }[] = [
		{ value: 'auto', label: 'network-ipv6-auto', description: 'network-ipv6-auto-description' },
		{ value: 'dhcp', label: 'network-ipv6-dhcp', description: 'network-ipv6-dhcp-description' },
		{ value: 'manual', label: 'network-manual', description: 'network-manual-description' },
		{ value: 'disabled', label: 'network-off', description: 'network-ipv6-off-description' }
	];

	$effect(() => {
		if (!interfaces.some((entry) => entry.name === selectedInterface) && interfaces.length > 0)
			selectedInterface = interfaces[0].name;
	});

	let selectedNetworkInterface = $derived(
		interfaces.find((item) => item.name === selectedInterface)
	);
	let selectedType = $derived(
		interfaceType(selectedNetworkInterface ?? { name: selectedInterface })
	);
	let isWireless = $derived(selectedType === 'wifi');
	// Tetra's network.configure writes Ethernet profiles only.
	let isEthernet = $derived(selectedType === 'ethernet');
	let currentAddresses = $derived(
		(selectedNetworkInterface?.addresses ?? [])
			.filter((entry) => entry.scope !== 'loopback')
			.map((entry) => `${entry.address}/${entry.prefix_length}`)
	);
	let selectedWifi = $derived(wifiNetworks.find((network) => network.ssid === wifiSsid));
	let bothOff = $derived(
		configure && isEthernet && ipv4.method === 'disabled' && ipv6.method === 'disabled'
	);
	let applying = $derived(apply?.status === 'applying' || apply?.status === 'waiting');
	let status = $derived<'unknown' | 'online' | 'offline'>(
		check ? ((check.ipv4 || check.ipv6) && check.dns ? 'online' : 'offline') : 'unknown'
	);

	$effect(() => {
		networkConfig =
			configure && isEthernet && selectedInterface
				? {
						interface: selectedInterface,
						ipv4: ipSettings(ipv4),
						ipv6: ipSettings(ipv6),
						...(wired8021x ? { wired_8021x: eap8021xSettings(wiredSettings) } : {})
					}
				: null;
	});

	$effect(() => {
		if (selectedWifi && selectedWifi.security !== 'unknown') {
			wifiSecurity = selectedWifi.security;
		}
	});

	async function scanWifi() {
		if (!onWifiScan || !selectedInterface) return;
		wifiLoading = true;
		wifiError = '';
		try {
			wifiNetworks = await onWifiScan(selectedInterface);
		} catch (error) {
			wifiError = error instanceof Error ? error.message : 'Wi-Fi scan failed';
		} finally {
			wifiLoading = false;
		}
	}

	async function connectWifi() {
		const ssid = hiddenSsid ? hiddenSsidName.trim() : wifiSsid;
		if (!onWifiConnect || !selectedInterface || !ssid) return;
		wifiLoading = true;
		wifiError = '';
		const security: WifiSecurityProfile =
			wifiSecurity === 'wpa-enterprise'
				? { type: 'wpa-enterprise', ...eap8021xSettings(wifiEnterprise) }
				: wifiSecurity === 'wpa-personal'
					? { type: 'wpa-personal', password: wifiPassword }
					: { type: 'open' };
		try {
			await onWifiConnect(selectedInterface, ssid, hiddenSsid, security);
		} catch (error) {
			wifiError = error instanceof Error ? error.message : 'Wi-Fi connection failed';
		} finally {
			wifiLoading = false;
		}
	}

	/** Enter in the network name or password connects, rather than continuing. */
	function connectOnEnter(event: KeyboardEvent) {
		if (event.key !== 'Enter' || event.isComposing) return;
		event.preventDefault();
		if (!wifiLoading && !applying && (wifiSsid || hiddenSsidName)) void connectWifi();
	}

	async function runCheck() {
		if (!onCheck) return;
		checking = true;
		checkError = '';
		try {
			check = await onCheck();
		} catch (error) {
			check = null;
			checkError = error instanceof Error ? error.message : 'Network check failed';
		} finally {
			checking = false;
		}
	}
</script>

{#snippet ipFamily(
	family: 'ipv4' | 'ipv6',
	form: IpForm<string>,
	options: { value: string; label: string; description: string }[],
	examples: { address: string; gateway: string; dns: string }
)}
	<fieldset>
		<legend>{t(family === 'ipv4' ? 'ipv4-configuration' : 'ipv6-configuration')}</legend>
		{#each options as option (option.value)}
			<label class="toggle-option" for="{family}-{option.value}">
				<span>
					<strong>{t(option.label)}</strong>
					<small>{t(option.description)}</small>
				</span>
				<input
					id="{family}-{option.value}"
					type="radio"
					name="{family}-method"
					value={option.value}
					checked={form.method === option.value}
					onchange={() => (form.method = option.value)}
				/>
			</label>
		{/each}
		{#if form.method === 'manual'}
			<div class="network-fields">
				<label class="field-label" for="{family}-addresses"
					>{t('network-addresses')}<input
						id="{family}-addresses"
						bind:value={form.addresses}
						placeholder={examples.address}
						autocomplete="off"
						spellcheck="false"
						aria-describedby="{family}-addresses-help"
					/></label
				>
				<p class="field-help" id="{family}-addresses-help">
					{t('network-addresses-help', { example: examples.address })}
				</p>
				<label class="field-label" for="{family}-gateway"
					>{t('network-gateway')}<input
						id="{family}-gateway"
						bind:value={form.gateway}
						placeholder={examples.gateway}
						autocomplete="off"
						spellcheck="false"
					/></label
				>
				<label class="field-label" for="{family}-dns"
					>{t('network-dns-servers')}<input
						id="{family}-dns"
						bind:value={form.dns}
						placeholder={examples.dns}
						autocomplete="off"
						spellcheck="false"
					/></label
				>
			</div>
		{/if}
	</fieldset>
{/snippet}

<StepLayout title={t('internet-title')} {onBack} {onContinue} canContinue={!bothOff && !applying}>
	{#if apply}
		<NetworkApplyStatus {apply} />
	{/if}
	<Callout
		tone={status === 'online' ? 'success' : status === 'offline' ? 'warning' : 'neutral'}
		title={status === 'online'
			? t('network-ready-title')
			: status === 'offline'
				? t('network-offline-title')
				: t('network-unchecked-title')}
	>
		{#if check}
			<ul class="network-check">
				<li>{t(check.ipv4 ? 'network-check-ipv4-yes' : 'network-check-ipv4-no')}</li>
				<li>{t(check.ipv6 ? 'network-check-ipv6-yes' : 'network-check-ipv6-no')}</li>
				<li>{t(check.dns ? 'network-check-dns-yes' : 'network-check-dns-no')}</li>
			</ul>
		{/if}
		<p>
			{status === 'offline' ? t('network-offline-description') : t('network-unchecked-description')}
		</p>
		{#if checkError}<p class="field-error" role="alert">{checkError}</p>{/if}
		<div class="inline-actions">
			<button
				class="secondary-button button-sm"
				type="button"
				onclick={runCheck}
				disabled={checking}
			>
				{checking ? t('checking-connection') : t('check-connection')}
			</button>
		</div>
	</Callout>
	<div class="network-settings">
		{#if interfaces.length > 0}
			<section class="network-section network-interface-section">
				<label class="field-label" for="network-interface"
					>{t('network-interface-label')}<select
						id="network-interface"
						bind:value={selectedInterface}
					>
						{#each interfaces as networkInterface}
							<option value={networkInterface.name}
								>{networkInterface.name} ({networkInterface.operstate ?? 'unknown'})</option
							>
						{/each}
					</select></label
				>
				<p class="field-help network-current-addresses">
					{t('network-current-addresses')}
					{currentAddresses.length ? currentAddresses.join(', ') : t('network-no-addresses')}
				</p>
				{#if isWireless}
					<fieldset data-enter-continue="off">
						<legend>{t('network-wifi-title')}</legend>
						<p class="field-help">{t('network-wifi-addressing')}</p>
						<div class="inline-actions">
							<button
								class="secondary-button button-sm"
								type="button"
								onclick={scanWifi}
								disabled={wifiLoading}
							>
								{wifiLoading ? t('network-wifi-scanning') : t('network-wifi-scan')}
							</button>
						</div>
						<label class="toggle-option" for="wifi-hidden">
							<span
								><strong>{t('network-wifi-hidden')}</strong><small
									>{t('network-wifi-hidden-description')}</small
								></span
							>
							<input id="wifi-hidden" type="checkbox" bind:checked={hiddenSsid} />
						</label>
						{#if hiddenSsid}
							<label class="field-label" for="wifi-hidden-ssid"
								>{t('network-wifi-ssid')}<input
									id="wifi-hidden-ssid"
									bind:value={hiddenSsidName}
									onkeydown={connectOnEnter}
								/></label
							>
						{:else if wifiNetworks.length > 0}
							<label class="field-label" for="wifi-network"
								>{t('network-wifi-network')}<select id="wifi-network" bind:value={wifiSsid}>
									<option value="">{t('network-wifi-select')}</option>
									{#each wifiNetworks as network}
										<option value={network.ssid}
											>{network.ssid} · {network.security} · {network.signal ?? 0}%</option
										>
									{/each}
								</select></label
							>
						{/if}
						{#if hiddenSsid || selectedWifi}
							<label class="field-label" for="wifi-security"
								>{t('network-wifi-security')}<select id="wifi-security" bind:value={wifiSecurity}>
									<option value="open">{t('network-wifi-open')}</option>
									<option value="wpa-personal">{t('network-wifi-wpa-personal')}</option>
									<option value="wpa-enterprise">{t('network-wifi-wpa-enterprise')}</option>
								</select></label
							>
						{/if}
						{#if (hiddenSsid || selectedWifi) && wifiSecurity === 'wpa-personal'}
							<label class="field-label" for="wifi-password"
								>{t('network-wifi-password')}<input
									id="wifi-password"
									type="password"
									bind:value={wifiPassword}
									onkeydown={connectOnEnter}
									autocomplete="current-password"
								/></label
							>
						{/if}
						{#if (hiddenSsid && wifiSecurity === 'wpa-enterprise') || (!hiddenSsid && selectedWifi?.security === 'wpa-enterprise' && wifiSecurity === 'wpa-enterprise')}
							<Eap8021xFields idPrefix="wifi" bind:settings={wifiEnterprise} />
						{/if}
						<div class="inline-actions">
							<button
								class="primary-button button-sm"
								type="button"
								onclick={connectWifi}
								disabled={wifiLoading || applying || (!wifiSsid && !hiddenSsidName)}
							>
								{t('network-wifi-connect')}
							</button>
						</div>
						{#if wifiError}<p class="field-error" role="alert">{wifiError}</p>{/if}
					</fieldset>
				{:else if !isEthernet}
					<p class="field-help network-other-interface">{t('network-other-interface')}</p>
				{:else}
					<label class="toggle-option" for="network-configure">
						<span>
							<strong>{t('network-configure')}</strong>
							<small>{t('network-configure-description')}</small>
						</span>
						<input id="network-configure" type="checkbox" bind:checked={configure} />
					</label>
					{#if configure}
						{@render ipFamily('ipv4', ipv4, ipv4Options, {
							address: '192.0.2.10/24',
							gateway: '192.0.2.1',
							dns: '1.1.1.1, 9.9.9.9'
						})}
						{@render ipFamily('ipv6', ipv6, ipv6Options, {
							address: '2001:db8::10/64',
							gateway: 'fe80::1',
							dns: '2606:4700:4700::1111'
						})}
						{#if bothOff}<p class="field-error" role="alert">{t('network-both-off')}</p>{/if}
						<fieldset>
							<legend>{t('network-wired-8021x-title')}</legend>
							<label class="toggle-option" for="wired-8021x">
								<span
									><strong>{t('network-wired-8021x-enable')}</strong><small
										>{t('network-wired-8021x-description')}</small
									></span
								>
								<input id="wired-8021x" type="checkbox" bind:checked={wired8021x} />
							</label>
							{#if wired8021x}
								<Eap8021xFields idPrefix="wired" bind:settings={wiredSettings} />
							{/if}
						</fieldset>
					{/if}
				{/if}
			</section>
		{/if}
	</div>
</StepLayout>

<style>
	.network-check li {
		color: var(--foreground);
	}

	.network-current-addresses {
		margin-top: -0.5rem;
		overflow-wrap: anywhere;
	}
</style>
