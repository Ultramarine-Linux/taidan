# shown on places like "keyboard layout variant: Default"
default = デフォルト
# btn (previous page)
prev = 前
# btn (next page)
next = 次
# shown on the progress bar
steps-dnfdownloadupdate = システムアップデートダウンロード中…
steps-dnfinstallupdate = システムアップデートインストール中…
steps-script = ディストリビューションスクリプト実行中…
steps-dnfdownloadapps = ユーザーアプリダウンロード中…
steps-dnfinstallapps = ユーザーアプリインストール中…
steps-driverscodecs = システムアップデートインストール中…
imelangs-ch = 中国語
imelangs-ja = 日本語
imelangs-ko = 韓国語
imelangs-vi = ベトナム語
imelangs-in = インド語群
imelangs-th = タイ語
imelangs-display = { $lang_name }（{ $native_lang_name }）

## page: Welcome

# page title
page-welcome = { $distro } へようこそ
page-welcome-ready = システムをセットアップしましょう。
page-welcome-go = 次
page-welcome-skipcfg = スキップ

## page: Keyboard Layout

# page title
page-keyboard = キーボード配列
page-keyboard-search-layout = キー配列を検索…
page-keyboard-search-variant = キーバリエーションを検索…

## page: Who are You?

# page title
# remember, if you can't translate things literally, think of what makes sense here as a page heading
page-whoareyou = ユーザー登録
page-whoareyou-fullname = 名前
page-whoareyou-username = ユーザー名
page-whoareyou-error =
    ユーザー名のフォーマットについて
    - 英小文字で始まる
    - 英文字・数字・<tt>_</tt>・<tt>-</tt>だけで組む

## page: Create a Password

# page title
# the `.pw` attribute is a text field placeholder
# the `.rp` attribute is yet another text field placeholder
page-password = パスワード作成
page-password-pw = パスワード
page-password-rp = 確認

## page: Internet

# page title
page-internet = インターネット接続
page-internet-desc = インターネットに接続すると、最新ソフトをダウンロードできるようになります。
page-internet-skip = スキップ
page-internet-warn = コーデック・デバイスドライバ・ユーザーアプリをインストールするにはネット接続が必要です。
page-internet-open = Wi-Fi接続アプリを起動

## page: Codecs and Drivers

page-codecs = コーデックとデバイスドライバ
page-codecs-desc1 = 専売特許のコーデック・デバイスドライバをインストールしますか？
page-codecs-desc2 = ネット接続が必要なので、接続のない場合は、{ $wiki }を読んでください。
page-codecs-wiki = ウィキの記事
switch-codecs = コーデック・デバイスドライバをインストール
switch-codecs-desc = 不必要である方は、「次」のボタンを押してください

## page: Input Method

# page title
page-inputmethod = 入力方法
page-inputmethod-desc1 = { $ime }の追加次第で、英語以外の入力が可能になりますが、設定はログインした後だけ有効になります。
page-inputmethod-desc2 =
    <b>日本語はMozcを選択してください。</b>

    特定の中国語入力方法は{ $rime }で利用可能かもしれませんが、セットアップが上級者向けのため、初心者におすすめしません。
page-inputmethod-desc3 = 詳細は{ $wiki }にご覧ください。
page-inputmethod-ime = IME
page-inputmethod-rime = Rime（中州韻輸入法引擎）
page-inputmethod-wiki = ウィキ
page-inputmethod-search-lang = 言語検索…
page-inputmethod-search-ims = 入力方法検索…

## page: Nightlight

# page title
page-nightlight = 夜間モード
page-nightlight-lbl1 = 画面の色合いを暖かくすることによって、目への負担を軽減することができます。
switch-nightlight = { page-nightlight }

## page: Choose Your Theme

# page title
page-theme = テーマ選択
page-theme-desc =
    色の綴でシステムをカスタマイズしましょう。
    後でシステム設定に変えることもできます。
page-theme-note = このオプションは従われない場合もあります。
page-theme-light = ライト
page-theme-dark = ダーク

## page: Browser Selection

page-browser = ブラウザ選択

## page: Categories
## this page shows different categories in the app catalogue (e.g. Productivity, Gaming, Media Prod., etc.)

page-categories = ユーザーアプリ
page-categories-confirm = 続きにセットアップ
page-categories-done = 完了
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

page-installing = アプリをインストール中
page-installing-desc = しばらくお待ちください。
page-installing-loading = ローディング…

## page: Finish

page-finish = 完成
page-finish-desc = 新しいシステムをお楽しみくださいませ。
page-finish-done = 完了

## page: Error

page-error = エラー
page-error-desc = 申し訳ございませんが、エラーが発生しました。
page-error-done = 閉じる
page-error-retry = リトライ
