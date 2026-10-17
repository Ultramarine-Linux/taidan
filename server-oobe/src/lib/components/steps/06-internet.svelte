<script lang="ts">
	import StepLayout from '$lib/components/StepLayout.svelte';
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
	let wifiEap = $state('peap');
	let wifiPhase2 = $state('mschapv2');
	let wifiDomain = $state('');
	let wifiUsername = $state('');
	let wifiAnonymousIdentity = $state('');
	let wifiCaCertificate = $state('');
	let wifiCaCertificateName = $state('');
	let wifiClientCertificate = $state('');
	let wifiPrivateKey = $state('');
	let wifiPrivateKeyPassword = $state('');
	let wired8021x = $state(false);
	let wiredEap = $state('peap');
	let wiredPhase2 = $state('mschapv2');
	let wiredDomain = $state('');
	let wiredUsername = $state('');
	let wiredAnonymousIdentity = $state('');
	let wiredCaCertificate = $state('');
	let wiredCaCertificateName = $state('');
	let wiredPassword = $state('');
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
				? {
						type: wifiSecurity,
						eap: wifiEap,
						phase2: wifiPhase2,
						domain: wifiDomain.trim() || undefined,
						username: wifiUsername.trim(),
						anonymousIdentity: wifiAnonymousIdentity.trim() || undefined,
						password: wifiPassword,
						caCertificate: wifiCaCertificate.trim() || undefined,
						caCertificateName: wifiCaCertificateName || undefined,
						clientCertificate: wifiClientCertificate.trim() || undefined,
						privateKey: wifiPrivateKey.trim() || undefined,
						privateKeyPassword: wifiPrivateKeyPassword || undefined
					}
				: {
						type: wifiSecurity,
						password: wifiSecurity === 'wpa-personal' ? wifiPassword : undefined
					};
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
			wired8021x:
				!isWireless && wired8021x
					? {
							enabled: true,
							eap: wiredEap,
							phase2: wiredPhase2,
							domain: wiredDomain.trim() || undefined,
							username: wiredUsername.trim(),
							anonymousIdentity: wiredAnonymousIdentity.trim() || undefined,
							caCertificate: wiredCaCertificate.trim() || undefined,
							caCertificateName: wiredCaCertificateName || undefined,
							password: wiredPassword
						}
					: { enabled: false }
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
						{#if (hiddenSsid && wifiSecurity !== 'open') || (!hiddenSsid && selectedWifi && wifiSecurity === 'wpa-personal')}
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
							<label class="field-label" for="wifi-eap">
								{t('network-wifi-eap')}<select id="wifi-eap" bind:value={wifiEap}>
									{#each selectedWifi?.eapMethods ?? ['peap', 'tls', 'ttls'] as method}<option
											value={method}>{method.toUpperCase()}</option
										>{/each}
								</select></label
							>
							<label class="field-label" for="wifi-phase2"
								>{t('network-wifi-phase2')}<select id="wifi-phase2" bind:value={wifiPhase2}>
									{#each selectedWifi?.phase2Methods ?? ['mschapv2', 'pap'] as method}<option
											value={method}>{method.toUpperCase()}</option
										>{/each}
								</select></label
							>
							<label class="field-label" for="wifi-domain"
								>{t('network-wifi-domain')}<input id="wifi-domain" bind:value={wifiDomain} /></label
							>
							<label class="field-label" for="wifi-username"
								>{t('network-wifi-username')}<input
									id="wifi-username"
									bind:value={wifiUsername}
									autocomplete="username"
								/></label
							>
							<label class="field-label" for="wifi-anonymous-identity"
								>{t('network-wifi-anonymous-identity')}<input
									id="wifi-anonymous-identity"
									bind:value={wifiAnonymousIdentity}
								/></label
							>
							<label class="field-label" for="wifi-ca-certificate"
								>{t('network-wifi-ca-certificate')}<input
									id="wifi-ca-certificate"
									bind:value={wifiCaCertificate}
								/></label
							>
							<div class="certificate-upload-row">
								<span class="certificate-upload-label">{t('network-wifi-ca-upload')}</span>
								<span class="certificate-upload-name"
									>{wifiCaCertificateName || t('network-wifi-no-certificate')}</span
								>
								<label class="certificate-browse" for="wifi-ca-upload"
									>{t('network-wifi-browse')}</label
								>
								<input
									id="wifi-ca-upload"
									class="certificate-file-input"
									type="file"
									accept=".pem,.crt,.cer,application/x-pem-file"
									onchange={async (event) => {
										const file = event.currentTarget.files?.[0];
										if (!file) return;
										wifiCaCertificate = await file.text();
										wifiCaCertificateName = file.name;
									}}
								/>
							</div>
							{#if wifiCaCertificateName}<p class="field-help">{wifiCaCertificateName}</p>{/if}
							<label class="field-label" for="wifi-enterprise-password"
								>{t('network-wifi-password')}<input
									id="wifi-enterprise-password"
									type="password"
									bind:value={wifiPassword}
									autocomplete="current-password"
								/></label
							>
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
							<label class="field-label" for="wired-eap"
								>{t('network-wifi-eap')}<select id="wired-eap" bind:value={wiredEap}
									><option value="peap">PEAP</option><option value="tls">TLS</option><option
										value="ttls">TTLS</option
									></select
								></label
							>
							<label class="field-label" for="wired-phase2"
								>{t('network-wifi-phase2')}<select id="wired-phase2" bind:value={wiredPhase2}
									><option value="mschapv2">MSCHAPv2</option><option value="pap">PAP</option
									></select
								></label
							>
							<label class="field-label" for="wired-domain"
								>{t('network-wifi-domain')}<input
									id="wired-domain"
									bind:value={wiredDomain}
								/></label
							>
							<label class="field-label" for="wired-username"
								>{t('network-wifi-username')}<input
									id="wired-username"
									bind:value={wiredUsername}
									autocomplete="username"
								/></label
							>
							<label class="field-label" for="wired-anonymous-identity"
								>{t('network-wifi-anonymous-identity')}<input
									id="wired-anonymous-identity"
									bind:value={wiredAnonymousIdentity}
								/></label
							>
							<label class="field-label" for="wired-ca-certificate"
								>{t('network-wifi-ca-certificate')}<input
									id="wired-ca-certificate"
									bind:value={wiredCaCertificate}
								/></label
							>
							<div class="certificate-upload-row">
								<span class="certificate-upload-label">{t('network-wifi-ca-upload')}</span>
								<span class="certificate-upload-name"
									>{wiredCaCertificateName || t('network-wifi-no-certificate')}</span
								>
								<label class="certificate-browse" for="wired-ca-upload"
									>{t('network-wifi-browse')}</label
								>
								<input
									id="wired-ca-upload"
									class="certificate-file-input"
									type="file"
									accept=".pem,.crt,.cer,application/x-pem-file"
									onchange={async (event) => {
										const file = event.currentTarget.files?.[0];
										if (!file) return;
										wiredCaCertificate = await file.text();
										wiredCaCertificateName = file.name;
									}}
								/>
							</div>
							{#if wiredCaCertificateName}<p class="field-help">{wiredCaCertificateName}</p>{/if}
							<label class="field-label" for="wired-password"
								>{t('network-wifi-password')}<input
									id="wired-password"
									type="password"
									bind:value={wiredPassword}
									autocomplete="current-password"
								/></label
							>
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
