export type KeyboardVariant = { id: string; name: string };
export type KeyboardLayout = { id: string; name: string; variants: KeyboardVariant[] };

export const keyboardLayouts: KeyboardLayout[] = [
	{
		id: 'al',
		name: 'Albanian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'plisi',
				name: 'Albanian (Plisi)'
			},
			{
				id: 'veqilharxhi',
				name: 'Albanian (Veqilharxhi)'
			}
		]
	},
	{
		id: 'et',
		name: 'Amharic',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'am',
		name: 'Armenian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'phonetic',
				name: 'Armenian (phonetic)'
			},
			{
				id: 'phonetic-alt',
				name: 'Armenian (alt. phonetic)'
			},
			{
				id: 'eastern',
				name: 'Armenian (eastern)'
			},
			{
				id: 'eastern-alt',
				name: 'Armenian (alt. eastern)'
			},
			{
				id: 'western',
				name: 'Armenian (western)'
			}
		]
	},
	{
		id: 'ara',
		name: 'Arabic',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'digits',
				name: 'Arabic (Eastern Arabic numerals)'
			},
			{
				id: 'azerty',
				name: 'Arabic (AZERTY)'
			},
			{
				id: 'azerty_digits',
				name: 'Arabic (AZERTY, Eastern Arabic numerals)'
			},
			{
				id: 'buckwalter',
				name: 'Arabic (Buckwalter)'
			},
			{
				id: 'mac',
				name: 'Arabic (Macintosh)'
			},
			{
				id: 'mac-phonetic',
				name: 'Arabic (Macintosh, phonetic)'
			},
			{
				id: 'olpc',
				name: 'Arabic (OLPC)'
			}
		]
	},
	{
		id: 'eg',
		name: 'Arabic (Egypt)',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'iq',
		name: 'Arabic (Iraq)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'ku',
				name: 'Kurdish (Iraq, Latin Q)'
			},
			{
				id: 'ku_alt',
				name: 'Kurdish (Iraq, Latin Alt-Q)'
			},
			{
				id: 'ku_f',
				name: 'Kurdish (Iraq, F)'
			},
			{
				id: 'ku_ara',
				name: 'Kurdish (Iraq, Arabic-Latin)'
			}
		]
	},
	{
		id: 'ma',
		name: 'Arabic (Morocco)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'tifinagh',
				name: 'Berber (Morocco, Tifinagh)'
			},
			{
				id: 'tifinagh-alt',
				name: 'Berber (Morocco, Tifinagh alt.)'
			},
			{
				id: 'tifinagh-alt-phonetic',
				name: 'Berber (Morocco, Tifinagh phonetic, alt.)'
			},
			{
				id: 'tifinagh-extended',
				name: 'Berber (Morocco, Tifinagh extended)'
			},
			{
				id: 'tifinagh-phonetic',
				name: 'Berber (Morocco, Tifinagh phonetic)'
			},
			{
				id: 'tifinagh-extended-phonetic',
				name: 'Berber (Morocco, Tifinagh extended phonetic)'
			},
			{
				id: 'french',
				name: 'French (Morocco)'
			},
			{
				id: 'rif',
				name: 'Tarifit'
			}
		]
	},
	{
		id: 'sy',
		name: 'Arabic (Syria)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'syc',
				name: 'Syriac'
			},
			{
				id: 'syc_phonetic',
				name: 'Syriac (phonetic)'
			},
			{
				id: 'ku',
				name: 'Kurdish (Syria, Latin Q)'
			},
			{
				id: 'ku_alt',
				name: 'Kurdish (Syria, Latin Alt-Q)'
			},
			{
				id: 'ku_f',
				name: 'Kurdish (Syria, F)'
			}
		]
	},
	{
		id: 'az',
		name: 'Azerbaijani',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'cyrillic',
				name: 'Azerbaijani (Cyrillic)'
			}
		]
	},
	{
		id: 'ml',
		name: 'Bambara',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'fr-oss',
				name: 'French (Mali, alt.)'
			},
			{
				id: 'us-mac',
				name: 'English (Mali, US, Macintosh)'
			},
			{
				id: 'us-intl',
				name: 'English (Mali, US, intl.)'
			}
		]
	},
	{
		id: 'bd',
		name: 'Bangla',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'probhat',
				name: 'Bangla (Probhat)'
			}
		]
	},
	{
		id: 'by',
		name: 'Belarusian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'legacy',
				name: 'Belarusian (legacy)'
			},
			{
				id: 'latin',
				name: 'Belarusian (Latin)'
			},
			{
				id: 'intl',
				name: 'Belarusian (intl.)'
			},
			{
				id: 'phonetic',
				name: 'Belarusian (phonetic)'
			},
			{
				id: 'ru',
				name: 'Russian (Belarus)'
			}
		]
	},
	{
		id: 'be',
		name: 'Belgian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'oss',
				name: 'Belgian (alt.)'
			},
			{
				id: 'oss_latin9',
				name: 'Belgian (Latin-9 only, alt.)'
			},
			{
				id: 'iso-alternate',
				name: 'Belgian (ISO, alt.)'
			},
			{
				id: 'nodeadkeys',
				name: 'Belgian (no dead keys)'
			},
			{
				id: 'wang',
				name: 'Belgian (Wang 724 AZERTY)'
			}
		]
	},
	{
		id: 'dz',
		name: 'Berber (Algeria, Latin)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'ber',
				name: 'Berber (Algeria, Tifinagh)'
			},
			{
				id: 'azerty-deadkeys',
				name: 'Kabyle (AZERTY, with dead keys)'
			},
			{
				id: 'qwerty-gb-deadkeys',
				name: 'Kabyle (QWERTY, UK, with dead keys)'
			},
			{
				id: 'qwerty-us-deadkeys',
				name: 'Kabyle (QWERTY, US, with dead keys)'
			},
			{
				id: 'ar',
				name: 'Arabic (Algeria)'
			}
		]
	},
	{
		id: 'ba',
		name: 'Bosnian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'alternatequotes',
				name: 'Bosnian (with guillemets)'
			},
			{
				id: 'unicode',
				name: 'Bosnian (with Bosnian digraphs)'
			},
			{
				id: 'unicodeus',
				name: 'Bosnian (US, with Bosnian digraphs)'
			},
			{
				id: 'us',
				name: 'Bosnian (US)'
			}
		]
	},
	{
		id: 'brai',
		name: 'Braille',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'left_hand',
				name: 'Braille (one-handed, left)'
			},
			{
				id: 'left_hand_invert',
				name: 'Braille (one-handed, left, inverted thumb)'
			},
			{
				id: 'right_hand',
				name: 'Braille (one-handed, right)'
			},
			{
				id: 'right_hand_invert',
				name: 'Braille (one-handed, right, inverted thumb)'
			}
		]
	},
	{
		id: 'bg',
		name: 'Bulgarian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'phonetic',
				name: 'Bulgarian (traditional phonetic)'
			},
			{
				id: 'bas_phonetic',
				name: 'Bulgarian (new phonetic)'
			},
			{
				id: 'bekl',
				name: 'Bulgarian (enhanced)'
			}
		]
	},
	{
		id: 'mm',
		name: 'Burmese',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'zawgyi',
				name: 'Burmese (Zawgyi)'
			},
			{
				id: 'mnw',
				name: 'Mon'
			},
			{
				id: 'mnw-a1',
				name: 'Mon (A1)'
			},
			{
				id: 'shn',
				name: 'Shan'
			},
			{
				id: 'zgt',
				name: 'Shan (Zawgyi)'
			}
		]
	},
	{
		id: 'cn',
		name: 'Chinese',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'altgr-pinyin',
				name: 'Hanyu Pinyin Letters (with AltGr dead keys)'
			},
			{
				id: 'mon_trad',
				name: 'Mongolian (Bichig)'
			},
			{
				id: 'mon_trad_todo',
				name: 'Mongolian (Todo)'
			},
			{
				id: 'mon_trad_xibe',
				name: 'Mongolian (Xibe)'
			},
			{
				id: 'mon_trad_manchu',
				name: 'Mongolian (Manchu)'
			},
			{
				id: 'mon_trad_galik',
				name: 'Mongolian (Galik)'
			},
			{
				id: 'mon_todo_galik',
				name: 'Mongolian (Todo Galik)'
			},
			{
				id: 'mon_manchu_galik',
				name: 'Mongolian (Manchu Galik)'
			},
			{
				id: 'tib',
				name: 'Tibetan'
			},
			{
				id: 'tib_asciinum',
				name: 'Tibetan (with ASCII numerals)'
			},
			{
				id: 'ug',
				name: 'Uyghur'
			}
		]
	},
	{
		id: 'hr',
		name: 'Croatian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'alternatequotes',
				name: 'Croatian (with guillemets)'
			},
			{
				id: 'unicode',
				name: 'Croatian (with Croatian digraphs)'
			},
			{
				id: 'unicodeus',
				name: 'Croatian (US, with Croatian digraphs)'
			},
			{
				id: 'us',
				name: 'Croatian (US)'
			}
		]
	},
	{
		id: 'cz',
		name: 'Czech',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'bksl',
				name: 'Czech (extra backslash)'
			},
			{
				id: 'qwerty',
				name: 'Czech (QWERTY)'
			},
			{
				id: 'qwerty_bksl',
				name: 'Czech (QWERTY, extra backslash)'
			},
			{
				id: 'winkeys',
				name: 'Czech (QWERTZ, Windows)'
			},
			{
				id: 'winkeys-qwerty',
				name: 'Czech (QWERTY, Windows)'
			},
			{
				id: 'qwerty-mac',
				name: 'Czech (QWERTY, Macintosh)'
			},
			{
				id: 'ucw',
				name: 'Czech (UCW, only accented letters)'
			},
			{
				id: 'dvorak-ucw',
				name: 'Czech (US, Dvorak, UCW support)'
			},
			{
				id: 'rus',
				name: 'Russian (Czechia, phonetic)'
			}
		]
	},
	{
		id: 'dk',
		name: 'Danish',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'Danish (no dead keys)'
			},
			{
				id: 'winkeys',
				name: 'Danish (Windows)'
			},
			{
				id: 'mac',
				name: 'Danish (Macintosh)'
			},
			{
				id: 'mac_nodeadkeys',
				name: 'Danish (Macintosh, no dead keys)'
			},
			{
				id: 'dvorak',
				name: 'Danish (Dvorak)'
			}
		]
	},
	{
		id: 'af',
		name: 'Dari',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'ps',
				name: 'Pashto'
			},
			{
				id: 'uz',
				name: 'Uzbek (Afghanistan)'
			},
			{
				id: 'fa-olpc',
				name: 'Dari (Afghanistan, OLPC)'
			},
			{
				id: 'ps-olpc',
				name: 'Pashto (Afghanistan, OLPC)'
			},
			{
				id: 'uz-olpc',
				name: 'Uzbek (Afghanistan, OLPC)'
			}
		]
	},
	{
		id: 'mv',
		name: 'Dhivehi',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'nl',
		name: 'Dutch',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'us',
				name: 'Dutch (US)'
			},
			{
				id: 'mac',
				name: 'Dutch (Macintosh)'
			},
			{
				id: 'std',
				name: 'Dutch (standard)'
			}
		]
	},
	{
		id: 'bt',
		name: 'Dzongkha',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'au',
		name: 'English (Australia)',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'cm',
		name: 'English (Cameroon)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'french',
				name: 'French (Cameroon)'
			},
			{
				id: 'qwerty',
				name: 'Cameroon Multilingual (QWERTY, intl.)'
			},
			{
				id: 'azerty',
				name: 'Cameroon (AZERTY, intl.)'
			},
			{
				id: 'dvorak',
				name: 'Cameroon (Dvorak, intl.)'
			},
			{
				id: 'mmuock',
				name: 'Mmuock'
			}
		]
	},
	{
		id: 'gh',
		name: 'English (Ghana)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'generic',
				name: 'English (Ghana, multilingual)'
			},
			{
				id: 'gillbt',
				name: 'English (Ghana, GILLBT)'
			},
			{
				id: 'akan',
				name: 'Akan'
			},
			{
				id: 'avn',
				name: 'Avatime'
			},
			{
				id: 'ewe',
				name: 'Ewe'
			},
			{
				id: 'fula',
				name: 'Fula'
			},
			{
				id: 'ga',
				name: 'Ga'
			},
			{
				id: 'hausa',
				name: 'Hausa (Ghana)'
			}
		]
	},
	{
		id: 'nz',
		name: 'English (New Zealand)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'mao',
				name: 'Maori'
			}
		]
	},
	{
		id: 'ng',
		name: 'English (Nigeria)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'hausa',
				name: 'Hausa (Nigeria)'
			},
			{
				id: 'igbo',
				name: 'Igbo'
			},
			{
				id: 'yoruba',
				name: 'Yoruba'
			}
		]
	},
	{
		id: 'za',
		name: 'English (South Africa)',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'gb',
		name: 'English (UK)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'extd',
				name: 'English (UK, extended, Windows)'
			},
			{
				id: 'intl',
				name: 'English (UK, intl., with dead keys)'
			},
			{
				id: 'dvorak',
				name: 'English (UK, Dvorak)'
			},
			{
				id: 'dvorakukp',
				name: 'English (UK, Dvorak, with UK punctuation)'
			},
			{
				id: 'mac',
				name: 'English (UK, Macintosh)'
			},
			{
				id: 'mac_intl',
				name: 'English (UK, Macintosh, intl.)'
			},
			{
				id: 'colemak',
				name: 'English (UK, Colemak)'
			},
			{
				id: 'colemak_dh',
				name: 'English (UK, Colemak-DH)'
			},
			{
				id: 'gla',
				name: 'Scottish Gaelic'
			},
			{
				id: 'pl',
				name: 'Polish (British keyboard)'
			}
		]
	},
	{
		id: 'us',
		name: 'English (US)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'euro',
				name: 'English (US, euro on 5)'
			},
			{
				id: 'intl',
				name: 'English (US, intl., with dead keys)'
			},
			{
				id: 'alt-intl',
				name: 'English (US, alt. intl.)'
			},
			{
				id: 'altgr-intl',
				name: 'English (intl., with AltGr dead keys)'
			},
			{
				id: 'mac',
				name: 'English (Macintosh, ABC, ANSI)'
			},
			{
				id: 'mac-iso',
				name: 'English (Macintosh, ABC, ISO)'
			},
			{
				id: 'colemak',
				name: 'English (Colemak)'
			},
			{
				id: 'colemak_dh',
				name: 'English (Colemak-DH)'
			},
			{
				id: 'colemak_dh_wide',
				name: 'English (Colemak-DH Wide)'
			},
			{
				id: 'colemak_dh_ortho',
				name: 'English (Colemak-DH Ortholinear)'
			},
			{
				id: 'colemak_dh_iso',
				name: 'English (Colemak-DH ISO)'
			},
			{
				id: 'colemak_dh_wide_iso',
				name: 'English (Colemak-DH Wide ISO)'
			},
			{
				id: 'dvorak',
				name: 'English (Dvorak)'
			},
			{
				id: 'dvorak-intl',
				name: 'English (Dvorak, intl., with dead keys)'
			},
			{
				id: 'dvorak-alt-intl',
				name: 'English (Dvorak, alt. intl.)'
			},
			{
				id: 'dvorak-l',
				name: 'English (Dvorak, one-handed, left)'
			},
			{
				id: 'dvorak-r',
				name: 'English (Dvorak, one-handed, right)'
			},
			{
				id: 'dvorak-classic',
				name: 'English (classic Dvorak)'
			},
			{
				id: 'dvp',
				name: 'English (programmer Dvorak)'
			},
			{
				id: 'dvorak-mac',
				name: 'English (Dvorak, Macintosh, ANSI)'
			},
			{
				id: 'dvorak-mac-iso',
				name: 'English (Dvorak, Macintosh, ISO)'
			},
			{
				id: 'norman',
				name: 'English (Norman)'
			},
			{
				id: 'symbolic',
				name: 'English (US, Symbolic)'
			},
			{
				id: 'workman',
				name: 'English (Workman)'
			},
			{
				id: 'workman-intl',
				name: 'English (Workman, intl., with dead keys)'
			},
			{
				id: 'olpc2',
				name: 'English (the divide/multiply toggle the layout)'
			},
			{
				id: 'chr',
				name: 'Cherokee'
			},
			{
				id: 'haw',
				name: 'Hawaiian'
			},
			{
				id: 'rus',
				name: 'Russian (US, phonetic)'
			},
			{
				id: 'hbs',
				name: 'Serbo-Croatian (US)'
			}
		]
	},
	{
		id: 'epo',
		name: 'Esperanto',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'legacy',
				name: 'Esperanto (legacy)'
			}
		]
	},
	{
		id: 'ee',
		name: 'Estonian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'Estonian (no dead keys)'
			},
			{
				id: 'dvorak',
				name: 'Estonian (Dvorak)'
			},
			{
				id: 'us',
				name: 'Estonian (US)'
			}
		]
	},
	{
		id: 'fo',
		name: 'Faroese',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'Faroese (no dead keys)'
			}
		]
	},
	{
		id: 'ph',
		name: 'Filipino',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'qwerty-bay',
				name: 'Filipino (QWERTY, Baybayin)'
			},
			{
				id: 'capewell-dvorak',
				name: 'Filipino (Capewell-Dvorak, Latin)'
			},
			{
				id: 'capewell-dvorak-bay',
				name: 'Filipino (Capewell-Dvorak, Baybayin)'
			},
			{
				id: 'capewell-qwerf2k6',
				name: 'Filipino (Capewell-QWERF 2006, Latin)'
			},
			{
				id: 'capewell-qwerf2k6-bay',
				name: 'Filipino (Capewell-QWERF 2006, Baybayin)'
			},
			{
				id: 'colemak',
				name: 'Filipino (Colemak, Latin)'
			},
			{
				id: 'colemak-bay',
				name: 'Filipino (Colemak, Baybayin)'
			},
			{
				id: 'dvorak',
				name: 'Filipino (Dvorak, Latin)'
			},
			{
				id: 'dvorak-bay',
				name: 'Filipino (Dvorak, Baybayin)'
			}
		]
	},
	{
		id: 'fi',
		name: 'Finnish',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'winkeys',
				name: 'Finnish (Windows)'
			},
			{
				id: 'classic',
				name: 'Finnish (classic)'
			},
			{
				id: 'nodeadkeys',
				name: 'Finnish (classic, no dead keys)'
			},
			{
				id: 'mac',
				name: 'Finnish (Macintosh)'
			},
			{
				id: 'smi',
				name: 'Northern Saami (Finland)'
			}
		]
	},
	{
		id: 'fr',
		name: 'French',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'French (no dead keys)'
			},
			{
				id: 'oss',
				name: 'French (alt.)'
			},
			{
				id: 'oss_nodeadkeys',
				name: 'French (alt., no dead keys)'
			},
			{
				id: 'oss_latin9',
				name: 'French (alt., Latin-9 only)'
			},
			{
				id: 'latin9',
				name: 'French (legacy, alt.)'
			},
			{
				id: 'latin9_nodeadkeys',
				name: 'French (legacy, alt., no dead keys)'
			},
			{
				id: 'azerty',
				name: 'French (AZERTY)'
			},
			{
				id: 'afnor',
				name: 'French (AZERTY, AFNOR)'
			},
			{
				id: 'bepo',
				name: 'French (BEPO)'
			},
			{
				id: 'bepo_latin9',
				name: 'French (BEPO, Latin-9 only)'
			},
			{
				id: 'bepo_afnor',
				name: 'French (BEPO, AFNOR)'
			},
			{
				id: 'dvorak',
				name: 'French (Dvorak)'
			},
			{
				id: 'ergol',
				name: 'French (Ergo‑L)'
			},
			{
				id: 'ergol_iso',
				name: 'French (Ergo‑L, ISO variant)'
			},
			{
				id: 'mac',
				name: 'French (Macintosh)'
			},
			{
				id: 'us',
				name: 'French (US)'
			},
			{
				id: 'bre',
				name: 'Breton (France)'
			},
			{
				id: 'oci',
				name: 'Occitan'
			},
			{
				id: 'geo',
				name: 'Georgian (France, AZERTY Tskapo)'
			}
		]
	},
	{
		id: 'ca',
		name: 'French (Canada)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'fr-dvorak',
				name: 'French (Canada, Dvorak)'
			},
			{
				id: 'fr-legacy',
				name: 'French (Canada, legacy)'
			},
			{
				id: 'multix',
				name: 'Canadian (CSA)'
			},
			{
				id: 'eng',
				name: 'English (Canada)'
			},
			{
				id: 'ike',
				name: 'Inuktitut'
			}
		]
	},
	{
		id: 'cd',
		name: 'French (Democratic Republic of the Congo)',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'tg',
		name: 'French (Togo)',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'ge',
		name: 'Georgian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'ergonomic',
				name: 'Georgian (ergonomic)'
			},
			{
				id: 'mess',
				name: 'Georgian (MESS)'
			},
			{
				id: 'os',
				name: 'Ossetian (Georgia)'
			},
			{
				id: 'ru',
				name: 'Russian (Georgia)'
			}
		]
	},
	{
		id: 'de',
		name: 'German',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'deadacute',
				name: 'German (dead acute)'
			},
			{
				id: 'deadgraveacute',
				name: 'German (dead grave acute)'
			},
			{
				id: 'deadtilde',
				name: 'German (dead tilde)'
			},
			{
				id: 'nodeadkeys',
				name: 'German (no dead keys)'
			},
			{
				id: 'e1',
				name: 'German (E1)'
			},
			{
				id: 'e2',
				name: 'German (E2)'
			},
			{
				id: 'T3',
				name: 'German (T3)'
			},
			{
				id: 'us',
				name: 'German (US)'
			},
			{
				id: 'dvorak',
				name: 'German (Dvorak)'
			},
			{
				id: 'mac',
				name: 'German (Macintosh)'
			},
			{
				id: 'mac_nodeadkeys',
				name: 'German (Macintosh, no dead keys)'
			},
			{
				id: 'neo',
				name: 'German (Neo 2)'
			},
			{
				id: 'qwerty',
				name: 'German (QWERTY)'
			},
			{
				id: 'dsb',
				name: 'Lower Sorbian'
			},
			{
				id: 'dsb_qwertz',
				name: 'Lower Sorbian (QWERTZ)'
			},
			{
				id: 'ro',
				name: 'Romanian (Germany)'
			},
			{
				id: 'ro_nodeadkeys',
				name: 'Romanian (Germany, no dead keys)'
			},
			{
				id: 'ru',
				name: 'Russian (Germany, phonetic)'
			},
			{
				id: 'tr',
				name: 'Turkish (Germany)'
			}
		]
	},
	{
		id: 'at',
		name: 'German (Austria)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'German (Austria, no dead keys)'
			},
			{
				id: 'mac',
				name: 'German (Austria, Macintosh)'
			}
		]
	},
	{
		id: 'ch',
		name: 'German (Switzerland)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'de_nodeadkeys',
				name: 'German (Switzerland, no dead keys)'
			},
			{
				id: 'de_mac',
				name: 'German (Switzerland, Macintosh)'
			},
			{
				id: 'legacy',
				name: 'German (Switzerland, legacy)'
			},
			{
				id: 'fr',
				name: 'French (Switzerland)'
			},
			{
				id: 'fr_nodeadkeys',
				name: 'French (Switzerland, no dead keys)'
			},
			{
				id: 'fr_mac',
				name: 'French (Switzerland, Macintosh)'
			}
		]
	},
	{
		id: 'gr',
		name: 'Greek',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'simple',
				name: 'Greek (simple)'
			},
			{
				id: 'nodeadkeys',
				name: 'Greek (no dead keys)'
			},
			{
				id: 'polytonic',
				name: 'Greek (polytonic)'
			}
		]
	},
	{
		id: 'il',
		name: 'Hebrew',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'si2',
				name: 'Hebrew (SI-1452-2)'
			},
			{
				id: 'lyx',
				name: 'Hebrew (lyx)'
			},
			{
				id: 'phonetic',
				name: 'Hebrew (phonetic)'
			},
			{
				id: 'biblical',
				name: 'Hebrew (Biblical, Tiro)'
			}
		]
	},
	{
		id: 'hu',
		name: 'Hungarian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'standard',
				name: 'Hungarian (standard)'
			},
			{
				id: 'nodeadkeys',
				name: 'Hungarian (no dead keys)'
			},
			{
				id: 'qwerty',
				name: 'Hungarian (QWERTY)'
			},
			{
				id: '101_qwertz_comma_dead',
				name: 'Hungarian (QWERTZ, 101-key, comma, dead keys)'
			},
			{
				id: '101_qwertz_comma_nodead',
				name: 'Hungarian (QWERTZ, 101-key, comma, no dead keys)'
			},
			{
				id: '101_qwertz_dot_dead',
				name: 'Hungarian (QWERTZ, 101-key, dot, dead keys)'
			},
			{
				id: '101_qwertz_dot_nodead',
				name: 'Hungarian (QWERTZ, 101-key, dot, no dead keys)'
			},
			{
				id: '101_qwerty_comma_dead',
				name: 'Hungarian (QWERTY, 101-key, comma, dead keys)'
			},
			{
				id: '101_qwerty_comma_nodead',
				name: 'Hungarian (QWERTY, 101-key, comma, no dead keys)'
			},
			{
				id: '101_qwerty_dot_dead',
				name: 'Hungarian (QWERTY, 101-key, dot, dead keys)'
			},
			{
				id: '101_qwerty_dot_nodead',
				name: 'Hungarian (QWERTY, 101-key, dot, no dead keys)'
			},
			{
				id: '102_qwertz_comma_dead',
				name: 'Hungarian (QWERTZ, 102-key, comma, dead keys)'
			},
			{
				id: '102_qwertz_comma_nodead',
				name: 'Hungarian (QWERTZ, 102-key, comma, no dead keys)'
			},
			{
				id: '102_qwertz_dot_dead',
				name: 'Hungarian (QWERTZ, 102-key, dot, dead keys)'
			},
			{
				id: '102_qwertz_dot_nodead',
				name: 'Hungarian (QWERTZ, 102-key, dot, no dead keys)'
			},
			{
				id: '102_qwerty_comma_dead',
				name: 'Hungarian (QWERTY, 102-key, comma, dead keys)'
			},
			{
				id: '102_qwerty_comma_nodead',
				name: 'Hungarian (QWERTY, 102-key, comma, no dead keys)'
			},
			{
				id: '102_qwerty_dot_dead',
				name: 'Hungarian (QWERTY, 102-key, dot, dead keys)'
			},
			{
				id: '102_qwerty_dot_nodead',
				name: 'Hungarian (QWERTY, 102-key, dot, no dead keys)'
			}
		]
	},
	{
		id: 'is',
		name: 'Icelandic',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'mac_legacy',
				name: 'Icelandic (Macintosh, legacy)'
			},
			{
				id: 'mac',
				name: 'Icelandic (Macintosh)'
			},
			{
				id: 'dvorak',
				name: 'Icelandic (Dvorak)'
			}
		]
	},
	{
		id: 'in',
		name: 'Indian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'asm-kagapa',
				name: 'Assamese (KaGaPa, phonetic)'
			},
			{
				id: 'ben',
				name: 'Bangla (India)'
			},
			{
				id: 'ben_probhat',
				name: 'Bangla (India, Probhat)'
			},
			{
				id: 'ben_baishakhi',
				name: 'Bangla (India, Baishakhi)'
			},
			{
				id: 'ben_bornona',
				name: 'Bangla (India, Bornona)'
			},
			{
				id: 'ben-kagapa',
				name: 'Bangla (India, KaGaPa, phonetic)'
			},
			{
				id: 'ben_gitanjali',
				name: 'Bangla (India, Gitanjali)'
			},
			{
				id: 'ben_inscript',
				name: 'Bangla (India, Baishakhi InScript)'
			},
			{
				id: 'eng',
				name: 'English (India, with rupee)'
			},
			{
				id: 'guj',
				name: 'Gujarati'
			},
			{
				id: 'guj-kagapa',
				name: 'Gujarati (KaGaPa, phonetic)'
			},
			{
				id: 'bolnagri',
				name: 'Hindi (Bolnagri)'
			},
			{
				id: 'hin-wx',
				name: 'Hindi (Wx)'
			},
			{
				id: 'hin-kagapa',
				name: 'Hindi (KaGaPa, phonetic)'
			},
			{
				id: 'kan',
				name: 'Kannada'
			},
			{
				id: 'kan-kagapa',
				name: 'Kannada (KaGaPa, phonetic)'
			},
			{
				id: 'mal',
				name: 'Malayalam'
			},
			{
				id: 'mal_lalitha',
				name: 'Malayalam (Lalitha)'
			},
			{
				id: 'mal_enhanced',
				name: 'Malayalam (enhanced InScript, with rupee)'
			},
			{
				id: 'mal_poorna',
				name: 'Malayalam (Poorna, extended InScript)'
			},
			{
				id: 'mni',
				name: 'Manipuri (Meitei)'
			},
			{
				id: 'mar-kagapa',
				name: 'Marathi (KaGaPa, phonetic)'
			},
			{
				id: 'marathi',
				name: 'Marathi (enhanced InScript)'
			},
			{
				id: 'ori',
				name: 'Oriya'
			},
			{
				id: 'ori-bolnagri',
				name: 'Oriya (Bolnagri)'
			},
			{
				id: 'ori-wx',
				name: 'Oriya (Wx)'
			},
			{
				id: 'guru',
				name: 'Punjabi (Gurmukhi)'
			},
			{
				id: 'jhelum',
				name: 'Punjabi (Gurmukhi Jhelum)'
			},
			{
				id: 'san-kagapa',
				name: 'Sanskrit (KaGaPa, phonetic)'
			},
			{
				id: 'sat',
				name: 'Santali (Ol Chiki)'
			},
			{
				id: 'tamilnet',
				name: "Tamil (TamilNet '99)"
			},
			{
				id: 'tamilnet_tamilnumbers',
				name: "Tamil (TamilNet '99 with Tamil numerals)"
			},
			{
				id: 'tamilnet_TAB',
				name: "Tamil (TamilNet '99, TAB encoding)"
			},
			{
				id: 'tamilnet_TSCII',
				name: "Tamil (TamilNet '99, TSCII encoding)"
			},
			{
				id: 'tam',
				name: 'Tamil (InScript, with Arabic numerals)'
			},
			{
				id: 'tam_tamilnumbers',
				name: 'Tamil (InScript, with Tamil numerals)'
			},
			{
				id: 'tel',
				name: 'Telugu'
			},
			{
				id: 'tel-kagapa',
				name: 'Telugu (KaGaPa, phonetic)'
			},
			{
				id: 'tel-sarala',
				name: 'Telugu (Sarala)'
			},
			{
				id: 'urd-phonetic',
				name: 'Urdu (phonetic)'
			},
			{
				id: 'urd-phonetic3',
				name: 'Urdu (alt. phonetic)'
			},
			{
				id: 'urd-winkeys',
				name: 'Urdu (Windows)'
			},
			{
				id: 'iipa',
				name: 'Indic IPA'
			}
		]
	},
	{
		id: 'id',
		name: 'Indonesian (Latin)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'melayu-phonetic',
				name: 'Indonesian (Arab Melayu, phonetic)'
			},
			{
				id: 'melayu-phoneticx',
				name: 'Indonesian (Arab Melayu, extended phonetic)'
			},
			{
				id: 'pegon-phonetic',
				name: 'Indonesian (Arab Pegon, phonetic)'
			},
			{
				id: 'javanese',
				name: 'Javanese'
			}
		]
	},
	{
		id: 'ie',
		name: 'Irish',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'UnicodeExpert',
				name: 'Irish (UnicodeExpert)'
			},
			{
				id: 'CloGaelach',
				name: 'CloGaelach'
			},
			{
				id: 'ogam',
				name: 'Ogham'
			},
			{
				id: 'ogam_is434',
				name: 'Ogham (IS434)'
			}
		]
	},
	{
		id: 'it',
		name: 'Italian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'Italian (no dead keys)'
			},
			{
				id: 'winkeys',
				name: 'Italian (Windows)'
			},
			{
				id: 'mac',
				name: 'Italian (Macintosh)'
			},
			{
				id: 'us',
				name: 'Italian (US)'
			},
			{
				id: 'ibm',
				name: 'Italian (IBM 142)'
			},
			{
				id: 'fur',
				name: 'Friulian (Italy)'
			},
			{
				id: 'scn',
				name: 'Sicilian'
			},
			{
				id: 'geo',
				name: 'Georgian (Italy)'
			}
		]
	},
	{
		id: 'jp',
		name: 'Japanese',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'kana',
				name: 'Japanese (Kana)'
			},
			{
				id: 'OADG109A',
				name: 'Japanese (OADG 109A)'
			},
			{
				id: 'mac',
				name: 'Japanese (Macintosh)'
			},
			{
				id: 'dvorak',
				name: 'Japanese (Dvorak)'
			}
		]
	},
	{
		id: 'kz',
		name: 'Kazakh',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'kazrus',
				name: 'Kazakh (with Russian)'
			},
			{
				id: 'ext',
				name: 'Kazakh (extended)'
			},
			{
				id: 'latin',
				name: 'Kazakh (Latin)'
			},
			{
				id: 'ruskaz',
				name: 'Russian (Kazakhstan, with Kazakh)'
			}
		]
	},
	{
		id: 'kh',
		name: 'Khmer (Cambodia)',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'kr',
		name: 'Korean',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'kr104',
				name: 'Korean (101/104-key compatible)'
			}
		]
	},
	{
		id: 'kg',
		name: 'Kyrgyz',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'phonetic',
				name: 'Kyrgyz (phonetic)'
			}
		]
	},
	{
		id: 'la',
		name: 'Lao',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'stea',
				name: 'Lao (STEA)'
			}
		]
	},
	{
		id: 'lv',
		name: 'Latvian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'apostrophe',
				name: 'Latvian (apostrophe)'
			},
			{
				id: 'tilde',
				name: 'Latvian (tilde)'
			},
			{
				id: 'fkey',
				name: 'Latvian (F)'
			},
			{
				id: 'modern',
				name: 'Latvian (Modern Latin)'
			},
			{
				id: 'modern-cyr',
				name: 'Latvian (Modern Cyrillic)'
			},
			{
				id: 'ergonomic',
				name: 'Latvian (ergonomic, ŪGJRMV)'
			},
			{
				id: 'adapted',
				name: 'Latvian (adapted)'
			}
		]
	},
	{
		id: 'lt',
		name: 'Lithuanian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'std',
				name: 'Lithuanian (standard)'
			},
			{
				id: 'us',
				name: 'Lithuanian (US)'
			},
			{
				id: 'ibm',
				name: 'Lithuanian (IBM)'
			},
			{
				id: 'lekp',
				name: 'Lithuanian (LEKP)'
			},
			{
				id: 'lekpa',
				name: 'Lithuanian (LEKPa)'
			},
			{
				id: 'ratise',
				name: 'Lithuanian (Ratise)'
			},
			{
				id: 'sgs',
				name: 'Samogitian'
			}
		]
	},
	{
		id: 'mk',
		name: 'Macedonian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'Macedonian (no dead keys)'
			}
		]
	},
	{
		id: 'my',
		name: 'Malay (Jawi, Arabic Keyboard)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'phonetic',
				name: 'Malay (Jawi, phonetic)'
			}
		]
	},
	{
		id: 'mt',
		name: 'Maltese',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'us',
				name: 'Maltese (US)'
			},
			{
				id: 'alt-us',
				name: 'Maltese (US, with AltGr overrides)'
			},
			{
				id: 'alt-gb',
				name: 'Maltese (UK, with AltGr overrides)'
			}
		]
	},
	{
		id: 'md',
		name: 'Moldavian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'gag',
				name: 'Gagauz (Moldova)'
			}
		]
	},
	{
		id: 'mn',
		name: 'Mongolian',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'me',
		name: 'Montenegrin',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'cyrillic',
				name: 'Montenegrin (Cyrillic)'
			},
			{
				id: 'cyrillicyz',
				name: 'Montenegrin (Cyrillic, ZE and ZHE swapped)'
			},
			{
				id: 'cyrillicalternatequotes',
				name: 'Montenegrin (Cyrillic, with guillemets)'
			},
			{
				id: 'latinunicode',
				name: 'Montenegrin (Latin, Unicode)'
			},
			{
				id: 'latinyz',
				name: 'Montenegrin (Latin, QWERTY)'
			},
			{
				id: 'latinunicodeyz',
				name: 'Montenegrin (Latin, Unicode, QWERTY)'
			},
			{
				id: 'latinalternatequotes',
				name: 'Montenegrin (Latin, with guillemets)'
			}
		]
	},
	{
		id: 'np',
		name: 'Nepali',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'gn',
		name: "N'Ko (AZERTY)",
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'no',
		name: 'Norwegian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'Norwegian (no dead keys)'
			},
			{
				id: 'winkeys',
				name: 'Norwegian (Windows)'
			},
			{
				id: 'mac',
				name: 'Norwegian (Macintosh)'
			},
			{
				id: 'mac_nodeadkeys',
				name: 'Norwegian (Macintosh, no dead keys)'
			},
			{
				id: 'colemak',
				name: 'Norwegian (Colemak)'
			},
			{
				id: 'colemak_dh',
				name: 'Norwegian (Colemak-DH)'
			},
			{
				id: 'colemak_dh_wide',
				name: 'Norwegian (Colemak-DH Wide)'
			},
			{
				id: 'dvorak',
				name: 'Norwegian (Dvorak)'
			},
			{
				id: 'smi',
				name: 'Northern Saami (Norway)'
			},
			{
				id: 'smi_nodeadkeys',
				name: 'Northern Saami (Norway, no dead keys)'
			}
		]
	},
	{
		id: 'ir',
		name: 'Persian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'pes_keypad',
				name: 'Persian (with Persian keypad)'
			},
			{
				id: 'winkeys',
				name: 'Persian (Windows)'
			},
			{
				id: 'azb',
				name: 'Azerbaijani (Iran)'
			},
			{
				id: 'ku',
				name: 'Kurdish (Iran, Latin Q)'
			},
			{
				id: 'ku_alt',
				name: 'Kurdish (Iran, Latin Alt-Q)'
			},
			{
				id: 'ku_f',
				name: 'Kurdish (Iran, F)'
			},
			{
				id: 'ku_ara',
				name: 'Kurdish (Iran, Arabic-Latin)'
			}
		]
	},
	{
		id: 'pl',
		name: 'Polish',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'legacy',
				name: 'Polish (legacy)'
			},
			{
				id: 'qwertz',
				name: 'Polish (QWERTZ)'
			},
			{
				id: 'dvorak',
				name: 'Polish (Dvorak)'
			},
			{
				id: 'dvorak_quotes',
				name: 'Polish (Dvorak, with Polish quotes on quotemark key)'
			},
			{
				id: 'dvorak_altquotes',
				name: 'Polish (Dvorak, with Polish quotes on key 1)'
			},
			{
				id: 'dvp',
				name: 'Polish (programmer Dvorak)'
			},
			{
				id: 'csb',
				name: 'Kashubian'
			},
			{
				id: 'szl',
				name: 'Silesian'
			},
			{
				id: 'ru_phonetic_dvorak',
				name: 'Russian (Poland, phonetic Dvorak)'
			}
		]
	},
	{
		id: 'pt',
		name: 'Portuguese',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'Portuguese (no dead keys)'
			},
			{
				id: 'mac',
				name: 'Portuguese (Macintosh)'
			},
			{
				id: 'mac_nodeadkeys',
				name: 'Portuguese (Macintosh, no dead keys)'
			},
			{
				id: 'nativo',
				name: 'Portuguese (Nativo)'
			},
			{
				id: 'nativo-us',
				name: 'Portuguese (Nativo for US keyboards)'
			},
			{
				id: 'nativo-epo',
				name: 'Esperanto (Portugal, Nativo)'
			}
		]
	},
	{
		id: 'br',
		name: 'Portuguese (Brazil)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'Portuguese (Brazil, no dead keys)'
			},
			{
				id: 'dvorak',
				name: 'Portuguese (Brazil, Dvorak)'
			},
			{
				id: 'nativo',
				name: 'Portuguese (Brazil, Nativo)'
			},
			{
				id: 'nativo-us',
				name: 'Portuguese (Brazil, Nativo for US keyboards)'
			},
			{
				id: 'thinkpad',
				name: 'Portuguese (Brazil, IBM/Lenovo ThinkPad)'
			},
			{
				id: 'nativo-epo',
				name: 'Esperanto (Brazil, Nativo)'
			},
			{
				id: 'rus',
				name: 'Russian (Brazil, phonetic)'
			}
		]
	},
	{
		id: 'ro',
		name: 'Romanian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'std',
				name: 'Romanian (standard)'
			},
			{
				id: 'winkeys',
				name: 'Romanian (Windows)'
			}
		]
	},
	{
		id: 'ru',
		name: 'Russian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'phonetic',
				name: 'Russian (phonetic)'
			},
			{
				id: 'phonetic_winkeys',
				name: 'Russian (phonetic, Windows)'
			},
			{
				id: 'phonetic_YAZHERTY',
				name: 'Russian (phonetic, YAZHERTY)'
			},
			{
				id: 'phonetic_azerty',
				name: 'Russian (phonetic, AZERTY)'
			},
			{
				id: 'phonetic_dvorak',
				name: 'Russian (phonetic, Dvorak)'
			},
			{
				id: 'typewriter',
				name: 'Russian (typewriter)'
			},
			{
				id: 'ruchey_ru',
				name: 'Russian (engineering, RU)'
			},
			{
				id: 'ruchey_en',
				name: 'Russian (engineering, EN)'
			},
			{
				id: 'legacy',
				name: 'Russian (legacy)'
			},
			{
				id: 'typewriter-legacy',
				name: 'Russian (typewriter, legacy)'
			},
			{
				id: 'dos',
				name: 'Russian (DOS)'
			},
			{
				id: 'mac',
				name: 'Russian (Macintosh)'
			},
			{
				id: 'ab',
				name: 'Abkhazian (Russia)'
			},
			{
				id: 'bak',
				name: 'Bashkirian'
			},
			{
				id: 'cv',
				name: 'Chuvash'
			},
			{
				id: 'cv_latin',
				name: 'Chuvash (Latin)'
			},
			{
				id: 'xal',
				name: 'Kalmyk'
			},
			{
				id: 'kom',
				name: 'Komi'
			},
			{
				id: 'chm',
				name: 'Mari'
			},
			{
				id: 'os_legacy',
				name: 'Ossetian (legacy)'
			},
			{
				id: 'os_winkeys',
				name: 'Ossetian (Windows)'
			},
			{
				id: 'srp',
				name: 'Serbian (Russia)'
			},
			{
				id: 'tt',
				name: 'Tatar'
			},
			{
				id: 'udm',
				name: 'Udmurt'
			},
			{
				id: 'sah',
				name: 'Yakut'
			}
		]
	},
	{
		id: 'rs',
		name: 'Serbian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'alternatequotes',
				name: 'Serbian (Cyrillic, with guillemets)'
			},
			{
				id: 'yz',
				name: 'Serbian (Cyrillic, ZE and ZHE swapped)'
			},
			{
				id: 'latin',
				name: 'Serbian (Latin)'
			},
			{
				id: 'latinalternatequotes',
				name: 'Serbian (Latin, with guillemets)'
			},
			{
				id: 'latinunicode',
				name: 'Serbian (Latin, Unicode)'
			},
			{
				id: 'latinyz',
				name: 'Serbian (Latin, QWERTY)'
			},
			{
				id: 'latinunicodeyz',
				name: 'Serbian (Latin, Unicode, QWERTY)'
			},
			{
				id: 'rue',
				name: 'Pannonian Rusyn'
			}
		]
	},
	{
		id: 'lk',
		name: 'Sinhala (phonetic)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'us',
				name: 'Sinhala (US)'
			},
			{
				id: 'tam_unicode',
				name: "Tamil (Sri Lanka, TamilNet '99)"
			},
			{
				id: 'tam_TAB',
				name: "Tamil (Sri Lanka, TamilNet '99, TAB encoding)"
			}
		]
	},
	{
		id: 'sk',
		name: 'Slovak',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'bksl',
				name: 'Slovak (extra backslash)'
			},
			{
				id: 'qwerty',
				name: 'Slovak (QWERTY)'
			},
			{
				id: 'qwerty_bksl',
				name: 'Slovak (QWERTY, extra backslash)'
			}
		]
	},
	{
		id: 'si',
		name: 'Slovenian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'alternatequotes',
				name: 'Slovenian (with guillemets)'
			},
			{
				id: 'us',
				name: 'Slovenian (US)'
			}
		]
	},
	{
		id: 'es',
		name: 'Spanish',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'Spanish (no dead keys)'
			},
			{
				id: 'deadtilde',
				name: 'Spanish (dead tilde)'
			},
			{
				id: 'winkeys',
				name: 'Spanish (Windows)'
			},
			{
				id: 'dvorak',
				name: 'Spanish (Dvorak)'
			},
			{
				id: 'ast',
				name: 'Asturian (Spain, with bottom-dot H and L)'
			},
			{
				id: 'cat',
				name: 'Catalan (Spain, with middle-dot L)'
			}
		]
	},
	{
		id: 'latam',
		name: 'Spanish (Latin American)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'Spanish (Latin American, no dead keys)'
			},
			{
				id: 'deadtilde',
				name: 'Spanish (Latin American, dead tilde)'
			},
			{
				id: 'dvorak',
				name: 'Spanish (Latin American, Dvorak)'
			},
			{
				id: 'colemak',
				name: 'Spanish (Latin American, Colemak)'
			}
		]
	},
	{
		id: 'ke',
		name: 'Swahili (Kenya)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'kik',
				name: 'Kikuyu'
			}
		]
	},
	{
		id: 'tz',
		name: 'Swahili (Tanzania)',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'se',
		name: 'Swedish',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'nodeadkeys',
				name: 'Swedish (no dead keys)'
			},
			{
				id: 'dvorak',
				name: 'Swedish (Dvorak)'
			},
			{
				id: 'us_dvorak',
				name: 'Swedish (Dvorak, intl.)'
			},
			{
				id: 'svdvorak',
				name: 'Swedish (Svdvorak)'
			},
			{
				id: 'colemak',
				name: 'Swedish (Colemak)'
			},
			{
				id: 'mac',
				name: 'Swedish (Macintosh)'
			},
			{
				id: 'us',
				name: 'Swedish (US)'
			},
			{
				id: 'swl',
				name: 'Swedish Sign Language'
			},
			{
				id: 'smi',
				name: 'Northern Saami (Sweden)'
			},
			{
				id: 'rus',
				name: 'Russian (Sweden, phonetic)'
			}
		]
	},
	{
		id: 'tw',
		name: 'Taiwanese',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'indigenous',
				name: 'Taiwanese (indigenous)'
			},
			{
				id: 'saisiyat',
				name: 'Saisiyat (Taiwan)'
			}
		]
	},
	{
		id: 'tj',
		name: 'Tajik',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'legacy',
				name: 'Tajik (legacy)'
			}
		]
	},
	{
		id: 'th',
		name: 'Thai',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'tis',
				name: 'Thai (TIS-820.2538)'
			},
			{
				id: 'pat',
				name: 'Thai (Pattachote)'
			},
			{
				id: 'mnc',
				name: 'Thai (Manoonchai)'
			}
		]
	},
	{
		id: 'bw',
		name: 'Tswana',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'tm',
		name: 'Turkmen',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'alt',
				name: 'Turkmen (Alt-Q)'
			}
		]
	},
	{
		id: 'tr',
		name: 'Turkish',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'f',
				name: 'Turkish (F)'
			},
			{
				id: 'e',
				name: 'Turkish (E)'
			},
			{
				id: 'alt',
				name: 'Turkish (Alt-Q)'
			},
			{
				id: 'intl',
				name: 'Turkish (intl., with dead keys)'
			},
			{
				id: 'ku',
				name: 'Kurdish (Turkey, Latin Q)'
			},
			{
				id: 'ku_f',
				name: 'Kurdish (Turkey, F)'
			},
			{
				id: 'ku_alt',
				name: 'Kurdish (Turkey, Latin Alt-Q)'
			}
		]
	},
	{
		id: 'ua',
		name: 'Ukrainian',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'phonetic',
				name: 'Ukrainian (phonetic)'
			},
			{
				id: 'typewriter',
				name: 'Ukrainian (typewriter)'
			},
			{
				id: 'winkeys',
				name: 'Ukrainian (Windows)'
			},
			{
				id: 'winkeysenhanced',
				name: 'Ukrainian (Windows Enhanced)'
			},
			{
				id: 'macOS',
				name: 'Ukrainian (macOS)'
			},
			{
				id: 'legacy',
				name: 'Ukrainian (legacy)'
			},
			{
				id: 'homophonic',
				name: 'Ukrainian (homophonic)'
			},
			{
				id: 'crh',
				name: 'Crimean Tatar (Turkish Q)'
			},
			{
				id: 'crh_f',
				name: 'Crimean Tatar (Turkish F)'
			},
			{
				id: 'crh_alt',
				name: 'Crimean Tatar (Turkish Alt-Q)'
			}
		]
	},
	{
		id: 'pk',
		name: 'Urdu (Pakistan)',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'urd-crulp',
				name: 'Urdu (Pakistan, CRULP)'
			},
			{
				id: 'urd-nla',
				name: 'Urdu (Pakistan, NLA)'
			},
			{
				id: 'pak_urdu_phonetic',
				name: 'Urdu (Pak Urdu Phonetic)'
			},
			{
				id: 'ara',
				name: 'Arabic (Pakistan)'
			},
			{
				id: 'snd',
				name: 'Sindhi'
			}
		]
	},
	{
		id: 'uz',
		name: 'Uzbek',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'latin',
				name: 'Uzbek (Latin)'
			}
		]
	},
	{
		id: 'vn',
		name: 'Vietnamese',
		variants: [
			{
				id: '',
				name: 'Default'
			},
			{
				id: 'us',
				name: 'Vietnamese (US)'
			},
			{
				id: 'fr',
				name: 'Vietnamese (France)'
			}
		]
	},
	{
		id: 'sn',
		name: 'Wolof',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	},
	{
		id: 'custom',
		name: 'A user-defined custom Layout',
		variants: [
			{
				id: '',
				name: 'Default'
			}
		]
	}
];
