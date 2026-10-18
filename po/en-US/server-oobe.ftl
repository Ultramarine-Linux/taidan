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
open-network-settings = Open network settings
network-interface-label = Network interface
ipv4-configuration = IPv4 configuration
network-use-dhcp = Use DHCP
network-dhcp-description = Configure the selected interface automatically.
network-static-ip = Static IPv4
network-static-description = Enter the address, gateway, and DNS servers.
network-ipv4-address = IPv4 address
network-prefix-length = Prefix length
network-gateway = Gateway
network-dns-servers = DNS servers

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
network-wifi-domain = Domain
network-wifi-username = Username
network-wifi-anonymous-identity = Anonymous identity
network-wifi-ca-certificate = CA certificate path (optional)
network-wifi-ca-upload = CA certificate (optional)
network-wifi-no-certificate = No certificate selected

# Internet — wired 802.1X
network-wired-8021x-title = Wired 802.1X
network-wired-8021x-enable = Use wired 802.1X
network-wired-8021x-description = Authenticate this Ethernet connection with enterprise credentials. Certificates are optional.

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
