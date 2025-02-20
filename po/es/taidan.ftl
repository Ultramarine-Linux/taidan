# shown on places like "keyboard layout variant: Default"
default = Por Defecto
# btn (previous page)
prev = Previo
# btn (next page)
next = Siguiente

# shown on the progress bar
steps =
    .dnfdownloadupdate = Descargando Actualización del Sistema…
    .dnfinstallupdate = Instalando Actualización del Sistema…
    .script =
    .dnfdownloadapps = Descargando Programas del Usuario…
    .dnfinstallapps = Instalando Programas del Usuario…
    .driverscodecs = Instalando controladores adicionales…

imelangs =
    .ch = Chino
    .ja = Japonés
    .ko = Coreano
    .vi = Vietnamita
    .in = 
    .th = Tailandés

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
page-welcome = Bienvenid@ a {$distro}
    .ready = Vamos a alistar tu sistema.
    # ≈ "Next" btn; on click, goes to the next page
    .go = Vamos
    # also a "Next" btn, but skips some pages
    .skipcfg = Saltar Configuración

## page: Keyboard Layout
# page title
page-keyboard = Layout de teclado
    # search field placeholder
    .search-layout = Busca tu layout de teclado…
    # search field placeholder
    .search-variant = Buscar tu variante de teclado…

## page: Who are You?
# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
page-whoareyou = Quién eres?
    # text field placeholder
    .fullname = Nombre Completo
    # text field placeholder
    .username = Nombre de Usuario
    # multiline, explanation pops out if invalid username
    .error = Nombre de Usuario
        - Iniciar con letras minúsculas
        - Solo contener caracteres alfanuméricos, guión bajo (<tt>_<tt>) o guión (<tt>-<tt>)

## page: Create a Password
# page title
# the `.pw` attribute is a text field placeholder
# the `.rp` attribute is yet another text field placeholder
page-password = Crear una contraseña
    .pw = Contraseña
    .rp = Repetir Contraseña

## page: Internet
# page title
page-internet = Vamos a ponerte en linea
    .desc = Conéctate al internet para obtener lo último y mejor.
    # btn
    .skip = No tengo Internet
    .warn = 
    # btn
    .open =

# do NOT translate (for now)
page-analytics =

# do NOT translate (for now)
page-crashreport =
    .desc =
    .lbl1 =
    .lbl2 =
# do NOT translate (for now)
# btn-switch
switch-crashreport =
    .desc =

# do NOT translate
page-location =
    .desc =

## page: Codecs and Drivers
page-codecs = Códecs y Controladores
    .desc1 = Instalar controladores y códecs multimedia propietarios para tu dispositivo.
    .desc2 = Consulta {$wiki} si no tienes una conexión a Internet.
    # we need to make this another attribute because we need to tag this as a link
    .wiki = wiki
switch-codecs = Instalar Códecs y Controladores
    .desc = Presiona Siguiente para saltar instalación

## page: Input Method
# page title
page-inputmethod = Método de Entrada
    .desc1 =
    .desc2 =
    .desc3 =
    .ime =
    .rime =
    .wiki =
    # search field placeholder
    .search-lang = Buscar lenguaje…
    # search field placeholder
    .search-ims = Buscar IMs/IMEs…

## page: Nightlight
# page title
page-nightlight = Luz Nocturna
    .lbl1 = Entinta la pantalla con un tono cálido en la noche para reducir cansancio ocular.
    .lbl2 = Luz Nocturna no está confirmado con ayudar con la dificultad de dormir.

switch-nightlight = Night Light

## page: Choose Your Theme
# page title
page-theme = Elige tu tema
    .desc = Haz este sistema el tuyo con un toque de color. Puedes cambiar esta opción después en Configuración.
    .note = Algunas aplicaciones podrían no respetar esta preferencia.
    # Light Theme
    .light = Claro
    # Dark Theme
    .dark = Oscuro

## page: Browser Selection
page-browser = Selección de Navegador

## page: Categories
## this page shows different categories in the app catalogue (e.g. Productivity, Gaming, Media Prod., etc.)
page-categories = Para que usarás este dispositivo?
    # btn
    .confirm = Confirmar y Configurar Sistema
    # btn; shown on new windows for closing
    .done = Hecho

# category names
# you should use a selector; e.g.
# { $cat ->
#   [Productivity] -> projdsalkaj;cjzkpqi
#   [...]
# }
# the list of categories is available at 
# https://github.com/Ultramarine-Linux/taidan/tree/master/catalogue
# open each yml file, check the first line for the name
categories = {$cat}

## page: Installing
page-installing = Instalando tus Aplicaciones
    .desc = Esto no tomará mucho!
    .loading = Cargando…

## page: Finish
page-finish = Tu Sistema está listo
    .desc = Esperamos que disfrutes tu nuevo sistema!
    # btn; closes the entire application
    .done = {$page-categories.done}

## page: Error
page-error = Error
    .desc = Lo sentimos, pero hay un problema inesperado.
    # btn; closes the entire application
    .done = {$page-categories.done}
    # btn
    .retry = Reintentar
