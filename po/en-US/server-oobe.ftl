# Ultramarine Server OOBE — shared interface
brand-name = Ultramarine Server
setup-title = Setup
fixture-mode = Fixture mode
get-server-ready = Get your server ready
prev = Back
next = Continue
review-setup = Review setup
go-to-dashboard = Go to Dashboard
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

# Step 7 — Server defaults
step-tweaks-label = Server defaults
step-tweaks-description = Choose which server services and policies to enable
defaults-title = Configure server defaults
defaults-description = Choose which services and policies should be enabled on this server.
defaults-ssh = Enable SSH
defaults-ssh-description = Allow remote administration over SSH.
defaults-firewall = Configure firewall
defaults-firewall-description = Allow SSH through the firewall when SSH is enabled.
automatic-updates = Enable automatic updates
automatic-updates-description = Enable the system update timer so security updates are applied automatically.

# Step 8 — Dashboard
step-dashboard-label = Dashboard
step-dashboard-description = Configure the local Dashboard address and port
hosting-title = Configure local Dashboard
hosting-local-description = Choose the domain and port where Dashboard will be hosted.
dashboard-domain = Dashboard domain
dashboard-domain-description = The hostname you'll use to open Dashboard.
dashboard-port = Dashboard port
dashboard-port-description = The local TCP port used by Dashboard.
dashboard-port-info = The default port is Fyra in T9!
dashboard-address-title = Dashboard address
dashboard-address-description = Configure where the local Dashboard listens.
dashboard-address-preview = Dashboard address
dashboard-domain-local = Local host

# Step 9 — Complete
step-complete-label = Complete
step-complete-description = Review setup and hand off to Dashboard
complete-title = Setup complete
complete-hostname = Hostname
complete-administrator = Administrator
complete-dashboard-domain = Dashboard domain
complete-dashboard-port = Dashboard port
