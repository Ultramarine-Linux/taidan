# Ultramarine Server OOBE — shared interface
brand-name = Ultramarine Server
setup-title = Setup
fixture-mode = Fixture mode
get-server-ready = Get your server ready
prev = Back
next = Continue
review-setup = Review setup
finish-setup = Finish setup
optional = optional
switch-to-dark = Switch to dark mode
switch-to-light = Switch to light mode
high-contrast = High contrast
open-steps = Show setup steps
close-steps = Hide setup steps
setup-steps = Setup steps
setup-sidebar = Setup progress
setup-progress-title = Server setup
setup-progress = { $completed } of { $total } steps complete
step-counter = Step { $current } of { $total }
step-complete-sr = Completed:
sidebar-footer = Finish setup to open the Dashboard.
loading-setup = Loading setup…
setup-state-unavailable = Unable to load setup state. Using defaults.
dismiss = Dismiss
retry = Retry
reboot = Reboot
shutdown = Shut down

# Step 1 — Welcome
step-welcome-label = Welcome
step-welcome-description = Welcome to Ultramarine Server
welcome-title = Welcome to Ultramarine Server
welcome-ready-title = Ready for setup
welcome-ready-description = Let's get your server ready to use
welcome-ip-addresses = Configure this server from another device at:
setup-code-label = Setup code
setup-code-description = Enter the setup code shown on this server's console. You can also print it there with `sudo tetra setup code`.
setup-code-fixture = Fixture mode accepts { $code }.
setup-code-invalid = That setup code is not correct. Check the server console and try again.
setup-code-rate-limited = Too many attempts. Try again in { $seconds } seconds.
setup-code-expired = Your setup session has ended. Enter the setup code again to continue.
setup-code-complete = Setup has already been completed on this server. Sign in to the Dashboard instead.
setup-code-unavailable = Cannot reach Tetra, the server's management service. Check that tetra.socket is running on the server.

# Step 2 — Device name
step-device-name-label = Device name
step-device-name-description = Name this server and set a hostname
device-name-title = Name your server
device-name-label = Device name
hostname-label = Hostname
hostname-error = Use lowercase letters, numbers, dots, and hyphens without empty labels.

# Step 3 — Administrator
step-administrator-label = Create a user
step-administrator-description = Create your administrator account
administrator-title = Create a user
full-name-label = Full name
username-label = Username
username-error = Use an all-lowercase username beginning with a letter or underscore.

# Step 4 — Password
step-password-label = Password
step-password-description = Set the administrator password
password-title = Set a password
password-label = Password
password-confirm-label = Confirm password
password-mismatch = Passwords do not match
password-too-short = Use at least 8 characters.

# Step 5 — SSH key
step-ssh-key-label = Public SSH key
step-ssh-key-description = Optionally add an SSH key for your user
ssh-key-title = Add an SSH key
ssh-key-label = Public SSH key
ssh-key-description = Optionally add an SSH key for your user
ssh-key-no-file = No key file selected
network-wifi-browse = Browse

# Step 6 — Internet and network
step-internet-label = Internet
step-internet-description = Configure network interfaces and connectivity
internet-title = Configure your network
network-ready-title = Network is ready
network-offline-title = No Internet connection
network-unchecked-title = Network status has not been checked
network-offline-description = You can continue setup offline and retry later.
network-unchecked-description = Select an interface and configure its connection.
check-connection = Check connection
checking-connection = Checking…
network-check-ipv4-yes = This server has an IPv4 address others can reach.
network-check-ipv4-no = No routable IPv4 address.
network-check-ipv6-yes = This server has an IPv6 address others can reach.
network-check-ipv6-no = No routable IPv6 address.
network-check-dns-yes = DNS resolves names.
network-check-dns-no = DNS doesn't resolve names.
network-interface-label = Network interface
network-current-addresses = Current addresses:
network-no-addresses = none
network-configure = Change this interface's settings
network-configure-description = Leave this off to keep the current settings.
ipv4-configuration = IPv4
ipv6-configuration = IPv6
network-ipv4-auto = Automatic (DHCP)
network-ipv4-auto-description = Get an address from the network's DHCP server.
network-ipv6-auto = Automatic
network-ipv6-auto-description = Use router advertisements (SLAAC), and DHCPv6 when the router asks for it.
network-ipv6-dhcp = DHCPv6
network-ipv6-dhcp-description = Get an address from a DHCPv6 server only.
network-manual = Manual
network-manual-description = Enter the addresses, gateway and DNS servers yourself.
network-off = Off
network-ipv4-off-description = For hosts that only have IPv6.
network-ipv6-off-description = Don't use IPv6 on this interface.
network-both-off = IPv4 and IPv6 can't both be off.
network-addresses = Addresses
network-addresses-help = With the prefix length, such as { $example }. Separate several with commas.
network-gateway = Gateway (optional)
network-dns-servers = DNS servers (optional)
network-wifi-addressing = Wi-Fi gets its IPv4 and IPv6 addresses automatically.
network-other-interface = Setup can only change the settings of Ethernet and Wi-Fi interfaces. Configure this one from the Dashboard or the console later.

# Internet — applying a change
network-apply-applying = Applying network settings…
network-apply-waiting = Waiting to reach setup again…
network-apply-kept = Network settings applied.
network-apply-rolled-back = Setup couldn't be reached with the new settings, so the server went back to its previous network settings.
network-apply-failed = The network settings couldn't be applied.
network-apply-urls = If this page loses its connection, open setup at:
network-apply-automatic = Addresses from DHCP or SLAAC can change. If these don't work, find the server's address on its console or your router.
network-apply-no-urls = The server's new address comes from DHCP or SLAAC. Find it on the server's console or your router.
network-apply-rollback = If setup can't be reached again within { $seconds } seconds, the server goes back to its previous network settings by itself.
network-change-kept = The new network settings are kept.

# Internet — Wi-Fi
network-wifi-title = Wi-Fi networks
network-wifi-scan = Scan for networks
network-wifi-scanning = Scanning…
network-wifi-network = Wi-Fi network
network-wifi-select = Select a network
network-wifi-hidden = Hidden network
network-wifi-hidden-description = Enter an SSID that is not advertised.
network-wifi-ssid = Network name (SSID)
network-wifi-security = Security
network-wifi-open = Open
network-wifi-wpa-personal = WPA-Personal
network-wifi-wpa-enterprise = WPA-Enterprise
network-wifi-password = Password
network-wifi-connect = Connect
network-wifi-eap = EAP method
network-wifi-phase2 = Inner authentication
network-wifi-anonymous-identity = Anonymous identity (optional)
network-wifi-no-certificate = No file selected
network-8021x-identity = Identity
network-8021x-domain = Server domain (optional)
network-8021x-domain-help = Only trust an authentication server whose certificate is for this domain or one of its subdomains.
network-8021x-ca-cert = CA certificate (optional)
network-8021x-client-cert = Client certificate
network-8021x-private-key = Private key
network-8021x-private-key-password = Private key password (optional)
network-8021x-not-pem = Choose a PEM file (text starting with -----BEGIN).

# Internet — wired 802.1X
network-wired-8021x-title = Wired 802.1X
network-wired-8021x-enable = Use wired 802.1X
network-wired-8021x-description = Authenticate this Ethernet connection with enterprise credentials.

# Step 7 — Updates
step-updates-label = Updates
step-updates-description = Choose how this server installs updates
updates-title = Choose an update policy
updates-description = Ultramarine Server can install updates on its own, or let you decide when. You can change this later in the Dashboard.
updates-policy = Update policy
updates-automatic = Automatic (recommended)
updates-automatic-description = Download and install updates automatically, including security fixes.
updates-notify = Notify only
updates-notify-description = Check for updates and show them in the Dashboard. Nothing is installed until you choose to.

# Step 8 — Remote access
step-remote-access-label = Remote access
step-remote-access-description = Optionally reach this server through Tailscale or Cloudflare Tunnel
remote-access-title = Set up remote access
remote-access-description = Both are off unless you turn them on here. You can also set them up later from the Dashboard.
tailscale-enable = Connect to Tailscale
tailscale-enable-description = Join this server to your tailnet so your devices can reach it privately.
tailscale-auth-key = Tailscale auth key
tailscale-auth-key-description = Create an auth key in the Tailscale admin console. It is passed to the server once and not stored by setup.
tailscale-serve = Share the Dashboard on your tailnet
tailscale-serve-description = Uses Tailscale Serve. Only devices on your tailnet can open it.
tailscale-funnel = Publish the Dashboard on the internet
tailscale-funnel-description = Uses Tailscale Funnel. Requires sharing on your tailnet.
tailscale-funnel-warning-title = Anyone on the internet will be able to reach the Dashboard
tailscale-funnel-warning = Funnel gives the Dashboard a public address. Only the sign-in page protects it, so use a strong password and set up two-factor authentication. Most servers only need Tailscale Serve.
tailscale-funnel-confirm = I understand the Dashboard will be public
cloudflared-enable = Connect a Cloudflare Tunnel
cloudflared-enable-description = Reach this server through a tunnel you created in Cloudflare, without opening ports.
cloudflared-token = Tunnel token
cloudflared-token-description = Copy the token from the tunnel's install command in Cloudflare Zero Trust. It is passed to the server once and not stored by setup.

# Step 9 — Dashboard access
step-dashboard-label = Dashboard
step-dashboard-description = Choose how browsers reach the Dashboard securely
access-title = Choose how to reach the Dashboard
access-description = The Dashboard is always served over HTTPS. Use a domain name for a trusted certificate, or this server's IP address.
access-mode = Address
access-domain = Domain name
access-domain-description = A trusted certificate from Let's Encrypt. Passkeys are available.
access-ip = IP address
access-ip-description = A certificate from this server's own authority. Browsers will warn until you trust it.
access-hostname = Domain name
access-challenge = Certificate verification
access-challenge-http = HTTP
access-challenge-http-description = The domain must point to this server, and ports 80 and 443 must be reachable from the internet.
access-challenge-dns = DNS
access-challenge-dns-description = Works behind a firewall or NAT. Needs an API token for your DNS provider.
access-dns-provider = DNS provider
access-dns-provider-select = Select a provider
access-dns-credentials-note = These credentials are passed to the server's management service, which keeps them for certificate renewals. Setup does not store them.
access-ip-address = IP address
access-ip-passkeys-title = Passkeys won't be available
access-ip-passkeys = Browsers only allow passkeys on domain names. You can still sign in with your password and an authenticator app, and switch to a domain later.
dashboard-address-preview = Dashboard address
dashboard-address-description = Open this address from another device after setup.

# Step 9 — Complete
step-complete-label = Complete
step-complete-description = Review setup and hand off to Dashboard
complete-title = Review and finish
complete-power-title = Power
complete-power-description = Reboot or shut down the server before finishing, if you need to.
complete-hostname = Hostname
complete-administrator = Administrator
complete-dashboard-address = Dashboard address
complete-updates = Updates
complete-remote-access = Remote access
complete-remote-none = Off
complete-remote-tailscale = Tailscale
complete-remote-tailscale-funnel = Tailscale with Funnel
complete-remote-cloudflared = Cloudflare Tunnel
complete-passkeys = Passkeys
complete-passkeys-yes = Available
complete-passkeys-no = Not available in IP mode
finished-title = Setup complete
finished-dashboard-starting = The Dashboard is starting
finished-handoff = This setup page will close in { $seconds } seconds. Sign in to the Dashboard with the account you created.
