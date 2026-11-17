use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Uk,
    Ru,
    De,
    Fr,
    Es,
    It,
    Ja,
    ZhHans,
    PtBr,
    Ko,
    Hi,
    Tr,
    Pl,
    Id,
}

impl Lang {
    pub const ALL: [Lang; 15] = [
        Lang::En,
        Lang::Uk,
        Lang::Ru,
        Lang::De,
        Lang::Fr,
        Lang::Es,
        Lang::It,
        Lang::Ja,
        Lang::ZhHans,
        Lang::PtBr,
        Lang::Ko,
        Lang::Hi,
        Lang::Tr,
        Lang::Pl,
        Lang::Id,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Uk => "uk",
            Lang::Ru => "ru",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::Es => "es",
            Lang::It => "it",
            Lang::Ja => "ja",
            Lang::ZhHans => "zh-Hans",
            Lang::PtBr => "pt-BR",
            Lang::Ko => "ko",
            Lang::Hi => "hi",
            Lang::Tr => "tr",
            Lang::Pl => "pl",
            Lang::Id => "id",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Uk => "Українська",
            Lang::Ru => "Русский",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::Es => "Español",
            Lang::It => "Italiano",
            Lang::Ja => "日本語",
            Lang::ZhHans => "简体中文",
            Lang::PtBr => "Português",
            Lang::Ko => "한국어",
            Lang::Hi => "हिन्दी",
            Lang::Tr => "Türkçe",
            Lang::Pl => "Polski",
            Lang::Id => "Bahasa Indonesia",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Lang::En => include_str!("../locales/en.json"),
            Lang::Uk => include_str!("../locales/uk.json"),
            Lang::Ru => include_str!("../locales/ru.json"),
            Lang::De => include_str!("../locales/de.json"),
            Lang::Fr => include_str!("../locales/fr.json"),
            Lang::Es => include_str!("../locales/es.json"),
            Lang::It => include_str!("../locales/it.json"),
            Lang::Ja => include_str!("../locales/ja.json"),
            Lang::ZhHans => include_str!("../locales/zh-Hans.json"),
            Lang::PtBr => include_str!("../locales/pt-BR.json"),
            Lang::Ko => include_str!("../locales/ko.json"),
            Lang::Hi => include_str!("../locales/hi.json"),
            Lang::Tr => include_str!("../locales/tr.json"),
            Lang::Pl => include_str!("../locales/pl.json"),
            Lang::Id => include_str!("../locales/id.json"),
        }
    }

    pub fn from_locale(locale: &str) -> Lang {
        let language = locale.split(['-', '_', '.']).next().unwrap_or("").to_ascii_lowercase();
        Lang::ALL
            .into_iter()
            .find(|lang| lang.code().split('-').next() == Some(language.as_str()))
            .unwrap_or(Lang::En)
    }

    pub fn forced() -> Option<Lang> {
        std::env::var("FRACTIQUE_LANG")
            .ok()
            .map(|locale| Lang::from_locale(&locale))
    }

    pub fn system() -> Lang {
        sys_locale::get_locale().map_or(Lang::En, |locale| Lang::from_locale(&locale))
    }
}

pub struct Texts {
    pub lang: Lang,
    own: HashMap<String, String>,
    english: HashMap<String, String>,
}

impl Texts {
    pub fn new(lang: Lang) -> Self {
        let read = |lang: Lang| serde_json::from_str(lang.source()).unwrap_or_default();
        Self {
            lang,
            own: read(lang),
            english: read(Lang::En),
        }
    }

    pub fn get(&self, key: &str) -> String {
        self.own
            .get(key)
            .or_else(|| self.english.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_every_text() {
        let english: HashMap<String, String> = serde_json::from_str(Lang::En.source()).unwrap();
        for lang in Lang::ALL {
            let own: HashMap<String, String> =
                serde_json::from_str(lang.source()).unwrap_or_else(|e| panic!("{}: {e}", lang.code()));
            let mut missing: Vec<&String> = english.keys().filter(|key| !own.contains_key(*key)).collect();
            missing.extend(own.keys().filter(|key| !english.contains_key(*key)));
            assert!(missing.is_empty(), "{}: {missing:?}", lang.code());
            assert!(own.values().all(|text| !text.trim().is_empty()), "{}", lang.code());
        }
    }

    #[test]
    fn locales_map_to_languages() {
        assert_eq!(Lang::from_locale("uk-UA"), Lang::Uk);
        assert_eq!(Lang::from_locale("pt_PT.UTF-8"), Lang::PtBr);
        assert_eq!(Lang::from_locale("zh-CN"), Lang::ZhHans);
        assert_eq!(Lang::from_locale("ja"), Lang::Ja);
        assert_eq!(Lang::from_locale("sv-SE"), Lang::En);
        assert_eq!(Lang::from_locale(""), Lang::En);
    }
}
