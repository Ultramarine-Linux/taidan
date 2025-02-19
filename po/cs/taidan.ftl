# shown on places like "keyboard layout variant: Default"
default = Výchozí
# btn (previous page)
prev = Předchozí
# btn (next page)
next = Další

# shown on the progress bar
steps =
    .dnfdownloadupdate = Stahování systémových aktualizací…
    .dnfinstallupdate = Instalace systémových aktualizací…
    .script =
    .dnfdownloadapps = Downloading User Programs…
    .dnfinstallapps = Installing User Programs…
    .driverscodecs = Installing additional drivers…

imelangs =
    .ch = Čínština
    .ja = Japonština
    .ko = Korejština
    .vi = Vietnamština
    .in = Indické
    .th = Thajština

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
    .display = {$lang_name} ({$native_lang_name})

## page: Welcome
# page title
page-welcome = Vítejte v {$distro}
    .ready = Připravme Váš systém.
    # ≈ "Next" btn; on click, goes to the next page
    .go = Pojďme
    # also a "Next" btn, but skips some pages
    .skipcfg = Přeskočit konfiguraci

## page: Keyboard Layout
# page title
page-keyboard = Rozložení klávesnice
    # search field placeholder
    .search-layout = Hledat rozložení klávesnice…
    # search field placeholder
    .search-variant = Hledat varianty rozložení klávesnice…

## page: Who are You?
# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
page-whoareyou = Kdo jste?
    # text field placeholder
    .fullname = Celé jméno
    # text field placeholder
    .username = Uživatelské jméno
    # multiline, explanation pops out if invalid username
    .error = Uživatelské jméno
        - musí začínat malým písmenem
        - musí obsahovat pouze písmena a čísla, podtržítko (<tt>_</tt>) nebo pomlčku (<tt>-</tt>)

## page: Create a Password
# page title
# the `.pw` attribute is a text field placeholder
# the `.rp` attribute is yet another text field placeholder
page-password = Vytvořte heslo
    .pw = Heslo
    .rp = Potvrďte heslo

## page: Internet
# page title
page-internet = Pojďme Vás připojit
    .desc = Připojte se k Internetu k získání toho nejnovějšího a nejlepšího.
    # btn
    .skip = Nemám připojení k Internetu
    .warn = Kodeky, ovladače a další uživatelské programy nebudou nainstalovány.
    # btn
    .open = Otevřít applet Wi-Fi připojení

# do NOT translate (for now)
page-analytics = Analýza a Hlášení nehod

# do NOT translate (for now)
page-crashreport = Hlášení nehod
    .desc = Povolte {$org} sbírání dat o pádech k odhalení chyb a Vaší pomoci.
    .lbl1 = All data collected is anonymous and end-to-end encrypted.
    .lbl2 = You will be given a crash ID to help support find what went wrong.
# do NOT translate (for now)
# btn-switch
switch-crashreport = Poslat data havárie
    .desc =

# do NOT translate
page-location = Služby určování polohy
    .desc = Povolit aplikacím žádato o Vaši přibližnou polohu s Mozilla Location Services

## page: Codecs and Drivers
page-codecs = Kodeky a ovladače
    .desc1 = 
    .desc2 = 
    # we need to make this another attribute because we need to tag this as a link
    .wiki = 
switch-codecs = Nainstalovat kodeky a ovladače
    .desc = Stiskněte další pro přeskočení instalace

## page: Input Method
# page title
page-inputmethod = Metody vstupu
    .desc1 =
    .desc2 =
    .desc3 =
    .ime =
    .rime =
    .wiki =
    # search field placeholder
    .search-lang = Hledat jazyk…
    # search field placeholder
    .search-ims = Hledat IMs/IMEs…

## page: Nightlight
# page title
page-nightlight = Noční osvětlení
    .lbl1 = V noci zbarví obrazovku teplým tónem, pro snížení únavy očí.
    .lbl2 = Není prokázáno, že by noční světlo pomáhalo při potížích s usínáním.

switch-nightlight = {$page-nightlight}

## page: Choose Your Theme
# page title
page-theme = Vyberte si motiv
    .desc = Přizpůsobte si system barevným nádechem.
            Tuto možnost si můžete později změnit v nastavení.
    .note = Některé aplikace nemusí toto nastavení respektovat.
    # Light Theme
    .light = Světlý
    # Dark Theme
    .dark = Tmavý

## page: Browser Selection
page-browser = Výběr prohlížeče

## page: Categories
## this page shows different categories in the app catalogue (e.g. Productivity, Gaming, Media Prod., etc.)
page-categories = K čemu toto zařízení využíváte?
    # btn
    .confirm = Potvrdit a nastavit systém
    # btn; shown on new windows for closing
    .done = Hotovo

# category names
# you should use a selector; e.g.
# { $cat ->
#   [Productivity] -> projdsalkaj;cjzkpqi
#   [...]
# }
# the list of categories is available at 
# https://github.com/Ultramarine-Linux/taidan/tree/master/catalogue
# open each yml file, check the first line for the name
categories =

## page: Installing
page-installing = Instalace Vašich aplikací
    .desc = Nepotrvá to dlouho!
    .loading = Načítání…

## page: Finish
page-finish = Váš systém je připraven
    .desc = Doufáme, že si svůj čerstvý systém užijete!
    # btn; closes the entire application
    .done = {$page-categories.done}

## page: Error
page-error = Chyba
    .desc = Omlouváme se, ale vyskytla se nečekaná chyba.
    # btn; closes the entire application
    .done = {$page-categories.done}
    # btn
    .retry = Opakovat
