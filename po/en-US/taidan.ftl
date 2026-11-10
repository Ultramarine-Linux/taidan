# shown on places like "keyboard layout variant: Default"
default = Default
# btn (previous page)
prev = Back
# btn (next page)
next = Next
# btn (dismiss a dialog without doing anything)
cancel = Cancel
# btn (skip anything)
skip = Skip
# btn (close a dialog after making changes in it)
done = Done
# window title, read out by screen readers and shown in window switchers
window-title = { $distro } Setup
# hidden menu entry for the Alt+Shift+T debug terminal, read out by screen readers
menu-open-terminal = Open Terminal
# hidden menu entry for the Meta+Alt+S shortcut, read out by screen readers
menu-toggle-screen-reader = Toggle Screen Reader
steps-useradd = Creating Your Account…
steps-sethostname = Naming Your Device…
steps-settimezone = Setting Your Time Zone and Clock…
steps-settheme = Applying Your Theme…
steps-dnfdownloadupdate = Downloading System Update…
steps-systemsetup = Setting Up Your System…
steps-dnfdownloadapps = Downloading Apps…
steps-dnfinstallapps = Installing Apps…
steps-driverscodecs = Installing Drivers and Codecs…
steps-setupimf = Setting Up Input Methods…
steps-setlocale = Applying Your Language…
# .display:
# This is intentional and you are required to fill in this field.
# $lang_name is the language name you probably have just translated (see
# the nearby strings above this). $native_lang_name is the language name
# in its corresponding language. For example, when displaying in English:
#
# Chinese (中文)
# Japanese (日本語)
# Korean (한국어)
# …
#
# In other languages, you might need to change the order around and maybe
# use different brackets, etc.
## [Accessibility]

# also the name of the button that opens it, in the header of every page
page-accessibility = Accessibility
page-accessibility-desc = Press the Feature to Enable
# tiles
page-accessibility-screen-reader = Screen Reader
page-accessibility-zoom = Zoom
page-accessibility-sticky-keys = Sticky Keys
# shown next to a picture of the accessibility button, which is in the top right corner of the
# later pages
page-accessibility-hint = You can come back to this page any time with the accessibility button in the top right corner.
# btn, opens the dialog with the rest of the options
page-accessibility-more = More Options
# btn, closes the page when it was opened with the accessibility button
page-accessibility-done = Done
page-accessibility-larger-text = Larger Text
page-accessibility-larger-text-desc = Make text and buttons bigger.
page-accessibility-legible-font = Legible Font
page-accessibility-legible-font-desc = Use Atkinson Hyperlegible, a typeface designed to tell letters apart for readers with low vision.
page-accessibility-reduce-motion = Reduce Motion
page-accessibility-reduce-motion-desc = Turn off animations.
page-accessibility-slow-keys = Slow Keys
page-accessibility-slow-keys-desc = Only accept a key if it's held down for a moment, so accidental presses are ignored.
page-accessibility-bounce-keys = Bounce Keys
page-accessibility-bounce-keys-desc = Ignore a key pressed again right after itself.
page-accessibility-dwell-click = Dwell Click
page-accessibility-dwell-click-desc = Click by holding the pointer still for a moment.
page-accessibility-color-filter = Color Filter
# .color-filter-*: options for the color filter, named like in Plasma's settings
page-accessibility-color-filter-none = None
page-accessibility-color-filter-protanopia = Protanopia (red weak)
page-accessibility-color-filter-deuteranopia = Deuteranopia (green weak)
page-accessibility-color-filter-tritanopia = Tritanopia (blue-yellow)
page-accessibility-color-filter-monochrome = Grayscale

# step-*: name of the current setup stage, shown above every page
step-account-creation = Account Creation
step-region-and-connectivity = Region and Connectivity
step-additional-software = Additional Software
step-finishing-up = Finishing Up

## [Language]

page-language = Language
page-language-search-lang = Search Language/Locale…

## [Welcome]

# page title
page-welcome-go = Let's Go
# big heading on the first page
page-welcome-hello = Welcome!
# .skipcfg: also a "Next" btn, but skips some pages
## [Keyboard Layout]

# page title
page-keyboard = Keyboard Layout
# .search*: search field placeholder
page-keyboard-search-layout = Search keyboard layouts…
page-keyboard-search-variant = Search keyboard variants…
# shown under the layouts when one that can't type Latin letters is picked, e.g. Russian
page-keyboard-latin-fallback = English (US) is added too, for typing names and passwords. Press Super+Space to switch between them.
# shown under the layouts when the picked layout couldn't be saved for the login screen
page-keyboard-not-saved = The layout couldn't be saved for the login screen yet. It will be tried again during setup.
# input methods (IMEs), for languages typed with more than a keyboard layout, e.g. Chinese
page-keyboard-ims = Input Methods
# what page-keyboard-ims shows when none are turned on, as in "Input Methods: None"
page-keyboard-ims-none = None
page-keyboard-ims-desc = For typing languages that need more than a keyboard layout, like Chinese, Japanese, or Korean. You can change these anytime in Settings.

## [Device Name]
## [Username]

# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
page-whoareyou = Who are You?
# .fullname: text field placeholder
page-whoareyou-fullname = Name
# .username: text field placeholder
page-whoareyou-username = Username
# .*-desc: help text below the field
page-whoareyou-fullname-desc = Your name, shown on the login screen.
page-whoareyou-username-desc = The name of your home folder.
# .icon: the picture shown for the user, e.g. on the login screen
page-whoareyou-icon = User Icon
# accessible name of each preset icon, $n counts from 1
page-whoareyou-icon-n = User Icon { $n }
page-whoareyou-icon-desc = Pick one, or take a photo.
# .camera: title of the dialog for taking the user icon with the webcam (also its button)
page-whoareyou-camera = Take a Photo
# btn
page-whoareyou-camera-take = Take Photo
# followed by the technical reason on the next line
page-whoareyou-camera-error = Can't use the camera:
# .error: multiline, explanation pops out if invalid username
page-whoareyou-error = Username must start with lowercase letters and must only contain lowercase letters, numbers, underscores (_), or hyphens (-).
page-whoareyou-error-taken = That username is already used on this system. Pick another one.
page-whoareyou-error-long = Usernames can be at most 31 characters long.
# shown under the name field if it has a comma or colon, which would cut the name short on the login screen
page-whoareyou-error-fullname = Names can't contain commas or colons.

## [Password]

# page title
page-password = Create a Password
# text field placeholder for password
page-password-pw = Password
# yet another text field placeholder for repeating the password.
# depending on the language you can just put the word equivalent to "repeat" here
page-password-rp = Repeat Password
# shown when the repeated password differs
page-password-mismatch = Passwords don't match.
# shown when the password is over 511 bytes, the most the system can store
page-password-long = This password is too long. Try a shorter one.
# accessible name of the button that shows/hides the password
page-password-show = Show password
# .tip*: advice shown next to the password fields
page-password-tip-title = Longer is Stronger
page-password-tip = A phrase is much better than number spaghetti.
# an example of a good password, keep it a short, memorable phrase joined by hyphens
page-password-tip-good = if-it-fits-i-sits
page-password-tip-good-desc = This takes centuries to crack, and you can remember it!
# an example of a bad password, keep it as is
page-password-tip-bad = Ds9Csga*XB
page-password-tip-bad-desc = Much harder to remember, and cracks in 12 days!

## [Time Zone]

page-timezone = Time Zone
page-timezone-search = Search for a city or time zone
page-timezone-none = No matching time zones.
# a city and the region or country it's in, e.g. "Toronto, Canada"
page-timezone-place = { $city }, { $region }

## [Internet]

# page title
page-internet = Get Online
page-internet-desc = Connect to the internet to install codecs, drivers, and apps.
# btn, opens a browser to log in to networks like hotel or airport wi-fi
page-internet-portal = Captive Portal
# btn, closes the browser opened by page-internet-portal
page-internet-portal-close = Close Portal
# btn, next to each wi-fi network
page-internet-connect = Connect
# shown next to the network in use
page-internet-connected = Connected
# btn, scans for wi-fi networks again
page-internet-refresh = Refresh
page-internet-scanning = Searching…
page-internet-no-wifi = No Wi-Fi adapter was found. You can still connect with a cable.
page-internet-no-networks = No Wi-Fi networks found.
page-internet-wifi-off = Wi-Fi is turned off.
page-internet-wifi-off-hardware = Wi-Fi is turned off with a switch or key on this device. Turn it on there to see networks.
# btn, shown with page-internet-wifi-off
page-internet-wifi-enable = Turn On Wi-Fi
# how a wi-fi network is secured, for screen readers and under networks that can't be used
page-internet-security-open = Open network
page-internet-security-password = Secured with a password
page-internet-security-enterprise = Secured with an organization login
page-internet-security-unsupported = Uses old WEP security, which isn't supported
# for screen readers, $security: one of the page-internet-security-* strings, $bars: signal strength from 0 to 3
page-internet-network-info = { $security }, signal strength { $bars } of 3
page-internet-wired = Wired Connection
# shown next to the wired connection when no cable is plugged in
page-internet-unplugged = Not Connected
# btn in the wired connection's settings, opens the 802.1X login
page-internet-8021x-open = 802.1X Security…
# dialog title, for wired networks with a login, like at work or school
page-internet-8021x-wired = Wired 802.1X Security
# btn, for networks that only reach the internet through a proxy server
page-internet-proxy = Proxy
# btn, the same as page-internet-proxy once one is set up
page-internet-proxy-on = Proxy ✓
page-internet-proxy-desc = Some networks, like at work or school, only reach the internet through a proxy server.
page-internet-proxy-server = Proxy Server
page-internet-proxy-bypass = Don't Use the Proxy For
page-internet-proxy-bypass-desc = Addresses and domains, separated by commas.
page-internet-proxy-invalid = Enter the proxy server like proxy.example.com:3128.
page-internet-proxy-error = Couldn't reach the internet through this proxy.
# toggle, off means the proxy is only used while setting up
page-internet-proxy-persist = Keep Using This Proxy After Setup
# btn, stops using the proxy
page-internet-proxy-remove = Remove
# btn, in the proxy and IP settings dialogs
page-internet-save = Save
# dialog title, and the label of the settings button next to each network; $name is the network
page-internet-ip = { $name } Settings
page-internet-ip-manual = Set the IP Address Manually
page-internet-ip-address = IP Address
page-internet-ip-gateway = Gateway
page-internet-ip-dns = DNS Servers
page-internet-ip-dns-desc = Separated by commas. Leave empty to use the network's.
page-internet-ip-dns-desc-manual = Separated by commas.
page-internet-ip-error-address = Enter the IP address like 192.168.1.20/24.
page-internet-ip-error-gateway = Enter the gateway like 192.168.1.1.
page-internet-ip-error-dns = Enter DNS servers like 1.1.1.1, 9.9.9.9.
page-internet-ip-error-dns-needed = Enter at least one DNS server.
page-internet-ip-error = Couldn't apply these settings.
page-internet-devicename = Device Name
page-internet-devicename-desc = How this device shows up to others on the network.
# $hostname: the name derived from the device name, e.g. janes-laptop
page-internet-hostname = Hostname: { $hostname }
# dialog title; $ssid is the name of the wi-fi network
page-internet-connect-to = Connect to { $ssid }
# opens a dialog for wi-fi networks that don't show up in the list
page-internet-hidden = Hidden Network…
page-internet-hidden-title = Connect to a Hidden Network
page-internet-hidden-ssid = Network Name
page-internet-hidden-security = Security
page-internet-hidden-wpa = WPA & WPA2 Personal
page-internet-hidden-wpa3 = WPA3 Personal
page-internet-hidden-open = None
page-internet-hidden-enterprise = WPA & WPA2 Enterprise
page-internet-password = Password
page-internet-error = Couldn't connect to this network.
page-internet-error-password = Couldn't connect. Check the password and try again.
page-internet-error-password-length = Wi-Fi passwords are 8 to 63 characters long.
page-internet-error-timeout = Connecting took too long. Try moving closer to the router.
# for 802.1X networks like eduroam, which take a username and password
page-internet-error-login = Couldn't connect. Check your username and password, or the server under Advanced.
page-internet-username = Username
page-internet-username-placeholder = name@university.edu
# btn, shows more settings for the network login
page-internet-advanced = Advanced
# label for picking how to log in to the network, e.g. PEAP or TTLS
page-internet-server = Authentication Server
page-internet-server-desc = Your password is only sent to a server with a valid certificate for this domain.
page-internet-server-desc-optional = Optional. Only connect to a server with a certificate for this domain.
# 802.1X login options, mirroring NetworkManager's connection editor
page-internet-8021x-method = Authentication
page-internet-8021x-peap = Protected EAP (PEAP)
page-internet-8021x-ttls = Tunneled TLS
page-internet-8021x-mschapv2-noeap = MSCHAPv2 (no EAP)
page-internet-8021x-identity = Identity
page-internet-8021x-inner = Inner Authentication
page-internet-8021x-peap-version = PEAP Version
page-internet-8021x-automatic = Automatic
page-internet-8021x-version = Version { $version }
page-internet-8021x-anonymous = Anonymous Identity
page-internet-8021x-anonymous-desc = Sent before the secure connection is set up, instead of your username.
page-internet-8021x-ca = Server Certificate
page-internet-8021x-ca-system = Check with the System's Certificates
page-internet-8021x-ca-file = Check with a CA Certificate File
page-internet-8021x-ca-none = Don't Check (Less Secure)
page-internet-8021x-ca-none-warn = Without checking the server, anyone pretending to be this network can see your login.
page-internet-8021x-ca-cert = CA Certificate
page-internet-8021x-client-cert = User Certificate
page-internet-8021x-private-key = Private Key
page-internet-8021x-key-password = Private Key Password
page-internet-8021x-key-password-desc = Leave empty if the key isn't protected with a password.
page-internet-8021x-fast-provisioning = Allow Automatic PAC Provisioning
page-internet-8021x-fast-anonymous = Anonymous
page-internet-8021x-fast-authenticated = Authenticated
page-internet-8021x-fast-both = Both
page-internet-8021x-pac = PAC File
page-internet-8021x-pac-desc = Optional when provisioning is allowed.
# btn, picks a file
page-internet-8021x-choose = Choose…
page-internet-8021x-clear = Remove File
page-internet-8021x-nofile = No file chosen
page-internet-8021x-error-tls = Couldn't connect. Check your identity, certificate and private key password.
page-internet-8021x-error-file = Couldn't copy the certificate files.
# file browser for certificates
page-internet-files = Choose a File
page-internet-files-drives = USB Drives
page-internet-files-home = Home
page-internet-files-root = Computer
page-internet-files-up = Parent Folder
page-internet-files-all = Show All Files
page-internet-files-empty = No certificates in this folder.
page-internet-files-empty-pac = No PAC files in this folder.
# dialog when continuing without internet
page-internet-offline = No Network Selected
page-internet-offline-desc = If you don't connect to the internet, drivers and codecs won't be installed.
# btn, skips everything that needs the internet, even if the device gets online later. Also confirms it in its dialog
page-internet-offline-choose = Set Up Offline
# dialog title, after page-internet-offline-choose
page-internet-offline-choose-title = Set Up Without Internet?
page-internet-offline-choose-desc = Drivers, codecs, and updates won't be installed, even if this device connects to the internet before setup finishes.

## [Analytics and Crash Reporting]
## this page is currently not in use, we recommend skipping

## [Tweaks]
page-tweaks = System Tweaks
page-tweaks-desc = Optional changes from your distribution. You can change them again later.
# btn, closes the tweaks dialog
page-tweaks-done = Done
## [Codecs and Drivers]

## [Input Method]

# page title
# languages that do require IMEs should change the second line to:
# Press "Add Input Method" to continue.
## [Choose Your Theme]

# page title
page-theme = Choose Your Theme
page-theme-light = Light
page-theme-dark = Dark
# the row of accent color swatches under the light and dark choices
page-theme-accent = Accent Color
# accent color names, read out by screen readers for each swatch
page-theme-accent-blue = Blue
page-theme-accent-teal = Teal
page-theme-accent-green = Green
page-theme-accent-yellow = Yellow
page-theme-accent-orange = Orange
page-theme-accent-red = Red
page-theme-accent-pink = Pink
page-theme-accent-purple = Purple
page-theme-accent-slate = Slate

## [Browser Selection]

## page: Categories

# page title
# category names
# you should use a selector; e.g.
# { $cat ->
#   [Productivity] -> projdsalkaj;cjzkpqi
#   […]
# }
## [Installing]

page-installing-configuring = Configuring your Device
# the step being done, and which one it is out of how many, e.g. "Creating your Account  (1/12)"
page-installing-stage = { $stage }  ({ $num }/{ $count })
## [Finish]

## [Location]

# also the toggle
page-location = Location Services
page-location-desc = Allow apps to request your approximate location with beaconDB.
page-location-tip-title = Where are you?
page-location-tip = We don't know. Location information from this feature is never shared with Fyra Labs, only with applications requesting it, like the weather widget, or a website you visit.
page-location-tip-control = You're in control, disable anytime in settings.

## [Drivers]

page-drivers = Drivers
page-drivers-desc = Install additional drivers, codecs, and tools to support hardware detected in this device.
page-drivers-proprietary = Some drivers may be proprietary.
# toggle
page-drivers-install = Install Drivers
# toggle, the same as page-drivers-install, when codecs are installed along with the drivers
page-drivers-install-codecs = Install Drivers and Codecs
# .nvidia, .broadcom-*, .builtin, .codecs: items in the list of drivers to install
page-drivers-nvidia = NVIDIA Graphics
page-drivers-broadcom-wifi = Broadcom WiFi
page-drivers-builtin = Built-in Drivers
page-drivers-codecs = Multimedia Codecs
# next to a QR code to a guide, when Secure Boot keeps the NVIDIA or Broadcom WiFi driver from being installed
page-drivers-secure-boot = Secure Boot is on, so these drivers can't be installed yet. Scan the code to learn how to set them up.

## [Summary]

# page title, $name is the user's first name
page-summary = Everything Look Right, { $name }?
page-summary-account = Your Account
page-summary-region = Region and Connectivity
page-summary-network = Network
page-summary-network-none = Not Connected
# page-internet-offline-choose was picked, so setup won't use the internet
page-summary-network-offline = Setting Up Offline
# the time zone's place, e.g. "Toronto, Canada", and its UTC offset, e.g. "UTC−05:00"
page-summary-timezone = { $place } ({ $offset })
page-summary-software = Additional Software
page-summary-software-none = No additional software will be installed.
# toggle, downloads updates during setup to install on the next restart
page-summary-updates = Download Updates
# toggle, $proxy is the proxy server's address. Off means the proxy is only used while setting up
page-summary-proxy = Keep Using the Proxy { $proxy }
# btn, opens the distribution's optional tweaks
page-summary-tweaks = Advanced Tweaks
# btn (next page), starts setting up the system
page-summary-confirm = Looks Good!

## [Done]

page-done = All Done
# btn, ends the setup and restarts, when the drivers need it
page-done-reboot = Restart
# btn, ends the setup and restarts to install the downloaded updates
page-done-restart-update = Restart and Update
page-done-updates = Updates are ready and will install the next time you restart
page-done-updates-drivers = Your drivers need a restart, which will install updates
page-done-drivers-failed = Some drivers couldn't be installed. Everything else is set up, and you can install them later.
# shown when the language pack for the chosen language isn't installed, e.g. when setting up offline
page-done-locale-failed = The system language couldn't be changed, likely because its language pack isn't installed. Everything else is set up, and you can change it later in Settings.
page-done-reboot-failed = Couldn't restart. Try again, or restart from the login screen.
# btn, ends the setup and shows the login screen, when no reboot is needed
page-done-login = Log In
# for screen readers, the spinner shown while the setup ends and the system restarts or shows the login screen
page-done-finishing = Finishing up…

## [Error]

page-error = Error
page-error-desc = We hit a snag.
# for screen readers, the box with the error message
page-error-details = Error Details
page-error-retry = Retry
# btn, retries without the steps that need the internet (updates, codecs, drivers, input methods, language packs)
page-error-offline = Continue Offline
# btn (back to the summary page to change settings before retrying)
page-error-back = Go Back
# btn, the setup starts over on the next boot
page-error-poweroff = Power Off

## [Tweaks in Ultramarine]

mtu-probing-name = MTU Probing
mtu-probing-desc = Enable MTU probing (fixes some applications like Ubisoft Connect). This can be reverted later.
cachyos-kernel-name = CachyOS Kernel
cachyos-kernel-desc = Install the CachyOS Kernel. This may provide a noticeable performance benefit, and can be reverted later.
