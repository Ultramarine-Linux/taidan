<script lang="ts">
	import Eap8021xFields from '$lib/components/Eap8021xFields.svelte';
	import StepLayout from '$lib/components/StepLayout.svelte';
	import {
		EAP_METHODS,
		PHASE2_METHODS,
		eap8021xSettings,
		emptyEap8021xForm,
		type EapMethod,
		type Phase2Method
	} from '$lib/eap-8021x';
	import { t } from '$lib/i18n.svelte';
	import type { NetworkInterface, WifiNetwork, WifiSecurityProfile } from '$lib/oobe-state';
	let status = $state<'unknown' | 'online' | 'offline'>('unknown');
	let checking = $state(false);
	let selectedInterface = $state('');
	let addressMode = $state<'dhcp' | 'static'>('dhcp');
	let staticAddress = $state('');
	let prefixLength = $state('24');
	let gateway = $state('');
	let dnsServers = $state('');
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
		networkConfig = $bindable<Record<string, unknown>>({}),
		onWifiScan = $bindable<(interfaceName: string) => Promise<WifiNetwork[]>>(),
		onWifiConnect = $bindable<
			(interfaceName: string, ssid: string, security: WifiSecurityProfile) => Promise<void>
		>()
	}: {
		onBack: () => void;
		onContinue: () => void;
		interfaces?: NetworkInterface[];
		networkConfig?: Record<string, unknown>;
		onWifiScan?: (interfaceName: string) => Promise<WifiNetwork[]>;
		onWifiConnect?: (
			interfaceName: string,
			ssid: string,
			security: WifiSecurityProfile
		) => Promise<void>;
	} = $props();

	$effect(() => {
		if (!selectedInterface && interfaces.length > 0) selectedInterface = interfaces[0].name;
		prepareConfig();
	});

	let selectedNetworkInterface = $derived(
		interfaces.find((item) => item.name === selectedInterface)
	);
	let isWireless = $derived(
		Boolean(selectedNetworkInterface?.wireless || selectedInterface.startsWith('wl'))
	);
	let selectedWifi = $derived(wifiNetworks.find((network) => network.ssid === wifiSsid));
	// Only offer methods Tetra accepts, whatever the scan reports.
	let wifiEapMethods = $derived(
		selectedWifi?.eapMethods?.filter((method): method is EapMethod =>
			(EAP_METHODS as readonly string[]).includes(method)
		)
	);
	let wifiPhase2Methods = $derived(
		selectedWifi?.phase2Methods?.filter((method): method is Phase2Method =>
			(PHASE2_METHODS as readonly string[]).includes(method)
		)
	);

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
			await onWifiConnect(selectedInterface, ssid, security);
		} catch (error) {
			wifiError = error instanceof Error ? error.message : 'Wi-Fi connection failed';
		} finally {
			wifiLoading = false;
		}
	}

	function prepareConfig() {
		networkConfig = {
			interface: selectedInterface,
			ipv4:
				addressMode === 'dhcp'
					? { method: 'dhcp' }
					: {
							method: 'static',
							address: staticAddress.trim(),
							prefix_length: Number(prefixLength),
							gateway: gateway.trim(),
							dns: dnsServers
								.split(',')
								.map((server) => server.trim())
								.filter(Boolean)
						},
			wired_8021x: !isWireless && wired8021x ? eap8021xSettings(wiredSettings) : undefined
		};
	}

	async function check() {
		checking = true;
		await new Promise((resolve) => setTimeout(resolve, 350));
		status = navigator.onLine ? 'online' : 'offline';
		checking = false;
	}
</script>

<StepLayout title={t('internet-title')} {onBack} {onContinue}>
	<div class="panel" class:success-panel={status === 'online'}>
		<div class="status-icon">{status === 'online' ? '✓' : status === 'offline' ? '!' : '…'}</div>
		<div>
			<h3>
				{status === 'online'
					? t('network-ready-title')
					: status === 'offline'
						? t('network-offline-title')
						: t('network-unchecked-title')}
			</h3>
			<p>
				{status === 'offline'
					? t('network-offline-description')
					: t('network-unchecked-description')}
			</p>
		</div>
	</div>
	<div class="network-settings">
		{#if interfaces.length > 0}
			<section class="network-section network-interface-section">
				<label class="field-label" for="network-interface"
					>{t('network-interface-label')}<select
						id="network-interface"
						bind:value={selectedInterface}
						onchange={prepareConfig}
					>
						{#each interfaces as networkInterface}
							<option value={networkInterface.name}
								>{networkInterface.name} ({networkInterface.state ?? 'unknown'})</option
							>
						{/each}
					</select></label
				>
				{#if isWireless}
					<fieldset>
						<legend>{t('network-wifi-title')}</legend>
						<div class="wifi-actions">
							<button
								class="secondary-button"
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
									autocomplete="current-password"
								/></label
							>
						{/if}
						{#if (hiddenSsid && wifiSecurity === 'wpa-enterprise') || (!hiddenSsid && selectedWifi?.security === 'wpa-enterprise' && wifiSecurity === 'wpa-enterprise')}
							<Eap8021xFields
								idPrefix="wifi"
								bind:settings={wifiEnterprise}
								eapMethods={wifiEapMethods?.length ? wifiEapMethods : undefined}
								phase2Methods={wifiPhase2Methods?.length ? wifiPhase2Methods : undefined}
							/>
						{/if}
						<button
							class="primary-button"
							type="button"
							onclick={connectWifi}
							disabled={wifiLoading || (!wifiSsid && !hiddenSsidName)}
						>
							{t('network-wifi-connect')}
						</button>
						{#if wifiError}<p class="field-error" role="alert">{wifiError}</p>{/if}
					</fieldset>
				{/if}
				<fieldset>
					<legend>{t('ipv4-configuration')}</legend>
					<label class="toggle-option" for="network-dhcp">
						<span>
							<strong>{t('network-use-dhcp')}</strong>
							<small>{t('network-dhcp-description')}</small>
						</span>
						<input
							id="network-dhcp"
							type="radio"
							name="ipv4-mode"
							checked={addressMode === 'dhcp'}
							onchange={() => {
								addressMode = 'dhcp';
								prepareConfig();
							}}
						/>
					</label>
					<label class="toggle-option" for="network-static">
						<span>
							<strong>{t('network-static-ip')}</strong>
							<small>{t('network-static-description')}</small>
						</span>
						<input
							id="network-static"
							type="radio"
							name="ipv4-mode"
							checked={addressMode === 'static'}
							onchange={() => {
								addressMode = 'static';
								prepareConfig();
							}}
						/>
					</label>
					{#if addressMode === 'static'}
						<div class="network-fields">
							<label class="field-label" for="static-address"
								>{t('network-ipv4-address')}<input
									id="static-address"
									bind:value={staticAddress}
									placeholder="192.168.1.20"
								/></label
							>
							<label class="field-label" for="prefix-length"
								>{t('network-prefix-length')}<input
									id="prefix-length"
									type="number"
									min="1"
									max="32"
									bind:value={prefixLength}
								/></label
							>
							<label class="field-label" for="gateway"
								>{t('network-gateway')}<input
									id="gateway"
									bind:value={gateway}
									placeholder="192.168.1.1"
								/></label
							>
							<label class="field-label" for="dns-servers"
								>{t('network-dns-servers')}<input
									id="dns-servers"
									bind:value={dnsServers}
									placeholder="1.1.1.1, 8.8.8.8"
								/></label
							>
						</div>
					{/if}
				</fieldset>
				{#if !isWireless}
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
			</section>
		{/if}
	</div>
	<div class="inline-actions">
		<button class="secondary-button" type="button" onclick={check} disabled={checking}>
			{checking ? t('checking-connection') : t('check-connection')}
		</button>
		{#if status === 'offline'}
			<button class="secondary-button" type="button" onclick={prepareConfig}>
				{t('open-network-settings')}
			</button>
		{/if}
	</div>
</StepLayout>
