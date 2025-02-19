# shown on places like "keyboard layout variant: Default"
default = Default

## page: Welcome

# page title
page-welcome = Welcome to {$distro}
    .ready = Let's get your system ready.
    # ≈ "Next" btn; on click, goes to the next page
    .go = Let's Go
    # also a "Next" btn, but skips some pages
    .skipcfg = Skip Configuration

## page: Keyboard Layout
# page title
page-keyboard = Keyboard Layout
    # search field placeholder
    .search-layout = Search keyboard layout…
    # search field placeholder
    .search-variant = Search keyboard variant…

## page: Who are You?
# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
page-whoareyou = Who are You?
    # text field placeholder
    .fullname = Full Name
    # text field placeholder
    .username = Username
    # multiline, explanation pops out if invalid username
    .error = Username must start with lowercase letters and contain only alphanumericals, underscore (<tt>_</tt>) or dash (<tt>-</tt>)

## page: Create a Password
# page title
# the `.pw` attribute is a text field placeholder
# the `.rp` attribute is yet another text field placeholder
page-password = Create a Password
    .pw = Password
    .rp = Repeat Password

## page: Internet
# page title
page-internet = Let's Get You Online
    .desc = Connect to the Internet to get the latest and greatest.
    # btn
    .skip = I don't have Internet
    .warn = Codecs, drivers and other user programs will not be installed.
    # btn
    .open = Open Wi-Fi connection applet

# do NOT translate (for now)
page-analytics = Analytics and Crash Reporting

# do NOT translate (for now)
page-crashreport = Crash Reporting
    .desc = Allow {$org} to collect crash data to find bugs and assist you.
    .lbl1 = All data collected is anonymous and end-to-end-encrypted.
    .lbl2 = You will be given a crash ID to help support find what went wrong.
# do NOT translate (for now)
# btn-switch
switch-crashreport = Send Crash Data
    .desc = Press next to keep off

# do NOT translate
page-location = Location Services
    .desc = Allow apps to request your approximate location with Mozilla Location Services.

## page: Codecs and Drivers
page-codecs = Codecs and Drivers
    .desc1 = Install proprietary media codecs and drivers for your device.
    .desc2 = Consult the {$wiki} if you don't have an Internet connection.
    # we need to make this another attribute because we need to tag this as a link
    .wiki = wiki
switch-codecs = Install Codecs and Drivers
    .desc = Press next to skip installation

## page: Input Method
# page title
page-inputmethod = Input Method
    .desc1 = You may <b>optionally</b> choose to add an {$ime}. This allows you to type in other specific languages. This change will take effect after you login inot your user account.
    .desc2 = More Chinese input method options may be available with the {$rime} engine, but this requires advanced configuration and is therefore not recommended to beginners.
    .desc3 = You may fine out more information on {$wiki}.
    .ime = input method editor (IME)
    .rime = Rime
    .wiki = the wiki
    # search field placeholder
    .search-lang = Search language…
    # search field placeholder
    .search-ims = Search IMs/IMEs…

## page: Nightlight
# page title
page-nightlight = Night Light
    .lbl1 = Tint the display with a warm tone at night to reduce eyestrain.
    .lbl2 = Night Light is not proven to help with difficulty falling sleep.

switch-nightlight = Night Light

## page: Choose Your Theme
# page title
page-theme = Choose Your Theme
    .desc = Make this system your own with a splash of colour.\nYou can change this option later in settings.
    .note = Some apps may not respect this preference.
    # Light Theme
    .light = Light
    .dark = Dark

## page: Browser Selection
page-browser = Browser Selection

## page: Categories
## this page shows different categories in the app catalogue (e.g. Productivity, Gaming, Media Prod., etc.)
page-categories = What Do You Use This Device For?
    # btn
    .confirm = Confirm and Setup System
    # btn; shown on new windows for closing
    .done = Done

# category names
# you should use a selector; e.g.
# { $cat ->
#   [Productivity] -> projdsalkaj;cjzkpqi
#   [...]
# }
# the list of categories is available at 
# https://github.com/Ultramarine-Linux/taidan/tree/master/catalogue
# open each yml file, check the first line for the name
categories = $cat

## page: Installing
page-installing = Installing Your Apps
    .desc = This won't take long!
    .loading = Loading…

## page: Finish
page-finish = Your System is Ready
    .desc = We hope you enjoy your fresh system!
    # btn; closes the entire application
    .done = Done

## page: Error
page-error = Error
    .desc = We are sorry, but there is an unexpected problem.
    # btn; closes the entire application
    .done = Done
    # btn
    .retry = Retry
    # btn
    .close = 
