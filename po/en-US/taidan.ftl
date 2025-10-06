# shown on places like "keyboard layout variant: Default"
default = Default
# btn (previous page)
prev = Previous
# btn (next page)
next = Next
# shown on the progress bar
steps-dnfdownloadupdate = Downloading System Update…
steps-dnfinstallupdate = Install System Update…
steps-script = Running Distribution scriptlet…
steps-dnfdownloadapps = Downloading User Programs…
steps-dnfinstallapps = Installing User Programs…
steps-driverscodecs = Installing additional drivers…
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
# ...
#
# In other languages, you might need to change the order around and maybe
# use different brackets, etc.
imelangs-display = { $lang_name } ({ $native_lang_name })

## page: Welcome

# .ready: ≈ "Next" btn; on click, goes to the next page
page-welcome = Welcome to { $distro }
# page title
page-welcome-ready = Let's get your system ready.
page-welcome-go = Let's Go
# .skipcfg: also a "Next" btn, but skips some pages
page-welcome-skipcfg = Skip Configuration

## page: Keyboard Layout

# page title
page-keyboard = Keyboard Layout
# .search*: search field placeholder
page-keyboard-search-layout = Search keyboard layout…
page-keyboard-search-variant = Search keyboard variant…

## page: Who are You?

# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
page-whoareyou = Who are You?
# .fullname: text field placeholder
page-whoareyou-fullname = Full Name
# .username: text field placeholder
page-whoareyou-username = Username
# .error: multiline, explanation pops out if invalid username
page-whoareyou-error = Username must start with lowercase letters and contain only alphanumericals, underscore (<tt>_</tt>) or dash (<tt>-</tt>)

## page: Create a Password

# page title
page-password = Create a Password
# the `.pw` attribute is a text field placeholder
page-password-pw = Password
# the `.rp` attribute is yet another text field placeholder
page-password-rp = Repeat Password

## page: Internet

# page title
# .skip: btn
# .open: btn
page-internet = Let's Get You Online
page-internet-desc = Connect to the Internet to get the latest and greatest.
page-internet-skip = I don't have Internet
page-internet-warn = Codecs, drivers and other user programs will not be installed.
page-internet-open = Open Wi-Fi connection applet
page-internet-ok = You are now connected to the Internet!
page-internet-portal = Open browser for web login
# do NOT translate (for now)
page-analytics = Analytics and Crash Reporting
# do NOT translate (for now)
page-crashreport = Crash Reporting
page-crashreport-desc = Allow { $org } to collect crash data to find bugs and assist you.
page-crashreport-lbl1 = All data collected is anonymous and end-to-end-encrypted.
page-crashreport-lbl2 = You will be given a crash ID to help support find what went wrong.
# do NOT translate (for now)
# btn-switch
switch-crashreport = Send Crash Data
switch-crashreport-desc = Press next to keep off

page-tweaks = System Tweaks

## page: Codecs and Drivers

page-codecs = Codecs and Drivers
page-codecs-desc1 = Install proprietary media codecs and drivers for your device.
page-codecs-desc2 =
    Consult the { $wiki } if you don't have an Internet connection.
page-codecs-wiki = wiki
switch-codecs = Install Codecs and Drivers
switch-codecs-desc = Press next to skip installation

## page: Input Method

# page title
# .search*: search field placeholder
page-inputmethod = Input Method
page-inputmethod-desc1 = You may <b>optionally</b> choose to add an { $ime }. This allows you to type in other specific languages. This change will take effect after you login into your user account.
page-inputmethod-desc2 = More Chinese input method options may be available with the { $rime } engine, but this requires advanced configuration and is therefore not recommended to beginners.
page-inputmethod-desc3 = You may fine out more information on { $wiki }.
page-inputmethod-ime = input method editor (IME)
page-inputmethod-rime = Rime
page-inputmethod-wiki = the wiki
page-inputmethod-search-lang = Search language…
page-inputmethod-search-ims = Search IMs/IMEs…

## page: Nightlight

# page title
page-nightlight = Night Light
page-nightlight-lbl1 = Tint the display with a warm tone at night to reduce eyestrain.
page-nightlight-lbl2 = Night Light is not proven to help with difficulty falling sleep.
switch-nightlight = Night Light

## page: Choose Your Theme

# page title
# .light: Light Theme
# .dark: Dark Theme
page-theme = Choose Your Theme
page-theme-desc =
    Make this system your own with a splash of colour.
    You can change this option later in settings.
page-theme-note = Some apps may not respect this preference.
page-theme-light = Light
page-theme-dark = Dark

## page: Browser Selection

page-browser = Browser Selection

## page: Categories
## this page shows different categories in the app catalogue (e.g. Productivity, Gaming, Media Prod., etc.)

# .confirm: btn
# .done: btn; shown on new windows for closing
page-categories = What Do You Use This Device For?
page-categories-confirm = Confirm and Setup System
page-categories-done = Done
# category names
# you should use a selector; e.g.
# { $cat ->
#   [Productivity] -> projdsalkaj;cjzkpqi
#   [...]
# }
# the list of categories is available at 
# https://github.com/Ultramarine-Linux/taidan/tree/master/catalogue
# open each yml file, check the first line for the name
categories = { $cat }

## page: Installing

page-installing = Installing Your Apps
page-installing-desc = This won't take long!
page-installing-loading = Loading…
page-installing-flatpak = Installing {$n} flatpaks…

## page: Finish

# .done: btn; closes the entire application
page-finish = Your System is Ready
page-finish-desc = We hope you enjoy your fresh system!
page-finish-done = Done

## page: Error

# .done: btn; closes the entire application
# .retry: btn
page-error = Error
page-error-desc = We are sorry, but there is an unexpected problem.
page-error-done = Done
page-error-retry = Retry

page-language = Language
page-language-search-lang = Search Language/Locale…

flatpakening-name = Use RPMs instead of Flatpaks
flatpakening-desc = Using RPMs may save space, but Flatpaks enhance system security by sandboxing designated applications.
