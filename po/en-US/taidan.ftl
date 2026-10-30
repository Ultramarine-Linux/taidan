# shown on places like "keyboard layout variant: Default"
default = Default
# btn (previous page)
prev = Back
# btn (next page)
next = Next
# btn (close a dialog)
close = Close
# btn (dismiss a dialog without doing anything)
cancel = Cancel
# btn (skip anything)
skip = Skip
steps-useradd = Creating Your Account…
steps-sethostname = Naming Your Device…
steps-settheme = Applying Your Theme…
steps-dnfdownloadupdate = Downloading System Update…
steps-dnfinstallupdate = Installing System Update…
steps-script = Running Scriptlet…
steps-dnfdownloadapps = Downloading Apps…
steps-dnfinstallapps = Installing Apps…
steps-driverscodecs = Installing Drivers and Codecs…
steps-setupimf = Setting Up Input Methods…
imelangs-ch = Chinese
imelangs-ja = Japanese
imelangs-ko = Korean
imelangs-vi = Vietnamese
imelangs-in = Indic
imelangs-th = Thai
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
imelangs-display = { $lang_name } ({ $native_lang_name })

# step-*: name of the current setup stage, shown above every page
step-account-creation = Account Creation
step-region-and-connectivity = Region and Connectivity
step-additional-software = Additional Software
step-finishing-up = Finishing Up

## [Language]

page-language = Language
page-language-search-lang = Search Language/Locale…

## [Welcome]

page-welcome = Welcome to { $distro }
# page title
page-welcome-ready = Let's get your device ready.
page-welcome-go = Let's Go
# big heading on the first page
page-welcome-hello = Welcome!
# .skipcfg: also a "Next" btn, but skips some pages
page-welcome-skipcfg = Skip Configuration

## [Keyboard Layout]

# page title
page-keyboard = Keyboard Layout
# .search*: search field placeholder
page-keyboard-search-layout = Search keyboard layouts…
page-keyboard-search-variant = Search keyboard variants…

## [Device Name]
page-devicename = Name This Device
page-devicename-devicename = Name
page-devicename-hostname = Hostname
page-devicename-error = Hostname must start with a letter, and may only contain lowercase letters, numbers, underscores (<tt>_</tt>), or hyphens (<tt>-</tt>).

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
page-whoareyou-icon-desc = Pick one, or take a photo.
# .camera: title of the dialog for taking the user icon with the webcam (also its button)
page-whoareyou-camera = Take a Photo
# btn
page-whoareyou-camera-take = Take Photo
# followed by the technical reason on the next line
page-whoareyou-camera-error = Can't use the camera:
# .error: multiline, explanation pops out if invalid username
page-whoareyou-error = Username must start with lowercase letters and must only contain lowercase letters, numbers, underscores (<tt>_</tt>), or hyphens (<tt>-</tt>).

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

## [Internet]

# page title
page-internet = Get Online
page-internet-desc = Connect to the internet to install codecs, drivers, and apps.
page-internet-skip = I Don't Have Internet
page-internet-warn = Codecs, drivers and some apps won't be installed without internet.
page-internet-open = Connect to a WiFi Network
page-internet-ok = You're connected! Press next to continue.
# btn, opens a browser to log in to networks like hotel or airport wi-fi
page-internet-portal = Captive Portal
# btn, next to each wi-fi network
page-internet-connect = Connect
# shown next to the network in use
page-internet-connected = Connected
# btn, scans for wi-fi networks again
page-internet-refresh = Refresh
page-internet-scanning = Searching…
page-internet-no-wifi = No Wi-Fi adapter was found. You can still connect with a cable.
page-internet-no-networks = No Wi-Fi networks found.
page-internet-wired = Wired Connection
page-internet-devicename = Device Name
page-internet-devicename-desc = How this device shows up to others on the network.
# $hostname: the name derived from the device name, e.g. janes-laptop
page-internet-hostname = Hostname: { $hostname }
# dialog title; $ssid is the name of the wi-fi network
page-internet-connect-to = Connect to { $ssid }
page-internet-password = Password
page-internet-error = Couldn't connect to this network.
page-internet-error-password = Couldn't connect. Check the password and try again.
page-internet-error-timeout = Connecting took too long. Try moving closer to the router.
# for networks like eduroam, which take a username and password
page-internet-error-login = Couldn't connect. Check your username and password, or the server under Advanced.
page-internet-username = Username
page-internet-username-placeholder = name@university.edu
# btn, shows more settings for the network login
page-internet-advanced = Advanced
# label for picking how to log in to the network, e.g. PEAP or TTLS
page-internet-method = Authentication
page-internet-server = Authentication Server
page-internet-server-desc = Your password is only sent to a server with a valid certificate for this domain.
page-internet-noverify = Don't verify the server (less secure)
# dialog when continuing without internet
page-internet-offline = No Network Selected
page-internet-offline-desc = If you don't connect to the internet, drivers and codecs won't be installed.

## [Analytics and Crash Reporting]
## this page is currently not in use, we recommend skipping

page-analytics = Analytics and Crash Reporting
page-crashreport = Crash Reporting
page-crashreport-desc = Allow { $org } to collect crash data to find bugs and assist you.
page-crashreport-lbl1 = All data collected is anonymous and end-to-end-encrypted.
page-crashreport-lbl2 = You will be given a crash ID to help support find what went wrong.
switch-crashreport = Send Crash Data
switch-crashreport-desc = Press next to keep off

## [Tweaks]
page-tweaks = System Tweaks
page-tweaks-more = Show advanced configurations

## [Codecs and Drivers]

page-codecs = Codecs and Drivers
page-codecs-desc1 = Install common media codecs and drivers for your device.
page-codecs-desc2 =
    Consult the { $wiki } if you don't have an Internet connection.
page-codecs-wiki = wiki
switch-codecs = Install Codecs and Drivers
switch-codecs-desc = Turning this on installs proprietary codecs and drivers

## [Input Method]

# page title
page-inputmethod = Additional Input Methods
# languages that do require IMEs should change the second line to:
# Press "Add Input Method" to continue.
page-inputmethod-desc =
    Languages with non-Latin characters may require an extra <b>input method editor (IME)</b>.
    You can skip this page if you do not need to type in one of these languages.
    More information is available on { $wiki }.
page-inputmethod-add = Add Input Method
page-inputmethod-wiki = the wiki
page-inputmethod-search-lang = Search language…
page-inputmethod-search-ims = Search IMs/IMEs…

## [Choose Your Theme]

# page title
page-theme = Choose Your Theme
page-theme-desc =
    Make this system your own with a splash of colour.
    You can change this option later in settings.
page-theme-note = Some apps may not respect this preference.
page-theme-light = Light
page-theme-dark = Dark

## [Browser Selection]

page-browser = Browser Selection

## page: Categories

# page title
page-categories = What Do You Use This Device For?
page-categories-confirm = Confirm and Setup System
page-categories-done = Done
# category names
# you should use a selector; e.g.
# { $cat ->
#   [Productivity] -> projdsalkaj;cjzkpqi
#   […]
# }
categories = { $cat }

## [Installing]

page-installing = Installing Your Apps
page-installing-configuring = Configuring your Device
page-installing-desc = This won't take long!
page-installing-loading = Loading…
page-installing-flatpak = Installing {$n} Flatpaks…

## [Finish]

## [Drivers]

page-drivers = Drivers
page-drivers-desc = Install additional drivers, codecs, and tools to support hardware detected in this device.
page-drivers-proprietary = Some drivers may be proprietary.
# toggle
page-drivers-install = Install Drivers
# .nvidia, .broadcom-*, .builtin: items in the list of drivers to install
page-drivers-nvidia = NVIDIA Graphics
page-drivers-broadcom-wifi = Broadcom WiFi
page-drivers-broadcom-bluetooth = Broadcom Bluetooth
page-drivers-builtin = Built-in Drivers

## [Summary]

# page title, $name is the user's first name
page-summary = Everything Look Right, { $name }?
page-summary-account = Your Account
page-summary-region = Region and Connectivity
page-summary-network = Network
page-summary-network-none = Not Connected
page-summary-software = Additional Software
page-summary-software-none = No additional software will be installed.
# btn (next page), starts setting up the system
page-summary-confirm = Looks Good!

## [Done]

page-done = All Done
page-done-reboot = Reboot
# btn, ends the setup and shows the login screen, when no reboot is needed
page-done-login = Log In

page-finish = Ready to Do Your Thing?
page-finish-desc = We hope you enjoy Ultramarine!
page-finish-done = Done

## [Error]

page-error = Error
page-error-desc = We hit a snag.
page-error-done = Exit
page-error-retry = Retry

## [Tweaks in Ultramarine]

flatpakening-name = Use RPMs instead of Flatpaks
flatpakening-desc = Flatpaks provide better security by default, RPMs will save some space.
mtu-probing-name = MTU Probing
mtu-probing-desc = Enable MTU probing (fixes some applications like Ubisoft Connect). This can be reverted later.
cachyos-kernel-name = CachyOS Kernel
cachyos-kernel-desc = Install the CachyOS Kernel. This may provide a noticeable performance benefit, and can be reverted later.
