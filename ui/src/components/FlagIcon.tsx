interface Props { code: string }

const MAP: Record<string, string> = {
  'en-US': 'us', 'en-GB': 'gb',
  'ru-RU': 'ru',
  'de-DE': 'de', 'fr-FR': 'fr', 'es-ES': 'es', 'it-IT': 'it',
  'pt-PT': 'pt', 'pl-PL': 'pl', 'uk-UA': 'ua', 'nl-NL': 'nl',
  'cs-CZ': 'cz', 'sv-SE': 'se', 'fi-FI': 'fi', 'ro-RO': 'ro',
  'hu-HU': 'hu', 'el-GR': 'gr', 'bg-BG': 'bg', 'da-DK': 'dk',
  'sk-SK': 'sk', 'lt-LT': 'lt', 'lv-LV': 'lv', 'et-EE': 'ee',
  'sl-SI': 'si', 'hr-HR': 'hr', 'no-NO': 'no', 'tr-TR': 'tr',
  'be-BY': 'by',
  // Asia
  'zh-CN': 'cn', 'zh-TW': 'cn', 'ja-JP': 'jp', 'ko-KR': 'kr',
  'hi-IN': 'in', 'ar-SA': 'sa', 'th-TH': 'th', 'vi-VN': 'vn',
  'id-ID': 'id', 'ms-MY': 'my', 'fa-IR': 'ir', 'he-IL': 'il',
  'bn-BD': 'bd', 'ur-PK': 'pk', 'ta-IN': 'in', 'te-IN': 'in',
  // Africa
  'sw-KE': 'ke', 'am-ET': 'et_af', 'zu-ZA': 'za', 'af-ZA': 'za',
  'ha-NG': 'ng', 'yo-NG': 'ng',
  // Americas
  'pt-BR': 'br',
};

/* Compact inline SVGs — rendered directly, not as data URIs */
const SVG: Record<string, string> = {
  us: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#B22234"/><g stroke="#fff" stroke-width="1.6"><line x1="0" y1="2.25" x2="24" y2="2.25"/><line x1="0" y1="6.75" x2="24" y2="6.75"/><line x1="0" y1="11.25" x2="24" y2="11.25"/><line x1="0" y1="15.75" x2="24" y2="15.75"/></g><rect width="10" height="9.75" fill="#3C3B6E"/></svg>',
  gb: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#012169"/><g stroke="#fff" stroke-width="2.5"><line x1="0" y1="0" x2="24" y2="18"/><line x1="24" y1="0" x2="0" y2="18"/></g><g stroke="#C8102E" stroke-width="1.5"><line x1="9" y1="0" x2="9" y2="18"/><line x1="15" y1="0" x2="15" y2="18"/><line x1="0" y1="7" x2="24" y2="7"/><line x1="0" y1="11" x2="24" y2="11"/></g><g stroke="#fff" stroke-width="4"><line x1="12" y1="0" x2="12" y2="18"/><line x1="0" y1="9" x2="24" y2="9"/></g><g stroke="#C8102E" stroke-width="2.5"><line x1="12" y1="0" x2="12" y2="18"/><line x1="0" y1="9" x2="24" y2="9"/></g></svg>',
  ru: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="6" fill="#fff"/><rect y="6" width="24" height="6" fill="#0039A6"/><rect y="12" width="24" height="6" fill="#D52B1E"/></svg>',
  de: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="6" fill="#000"/><rect y="6" width="24" height="6" fill="#DD0000"/><rect y="12" width="24" height="6" fill="#FFCE00"/></svg>',
  fr: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="8" height="18" fill="#002395"/><rect x="8" width="8" height="18" fill="#fff"/><rect x="16" width="8" height="18" fill="#ED2939"/></svg>',
  es: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="4.5" fill="#AA151B"/><rect y="4.5" width="24" height="9" fill="#F1BF00"/><rect y="13.5" width="24" height="4.5" fill="#AA151B"/></svg>',
  it: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="8" height="18" fill="#009246"/><rect x="8" width="8" height="18" fill="#fff"/><rect x="16" width="8" height="18" fill="#CE2B37"/></svg>',
  pt: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="8" height="18" fill="#006600"/><rect x="8" width="16" height="18" fill="#FF0000"/></svg>',
  pl: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="9" fill="#fff"/><rect y="9" width="24" height="9" fill="#DC143C"/></svg>',
  ua: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="9" fill="#0057B7"/><rect y="9" width="24" height="9" fill="#FFD700"/></svg>',
  nl: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="6" fill="#AE1C28"/><rect y="6" width="24" height="6" fill="#fff"/><rect y="12" width="24" height="6" fill="#21468B"/></svg>',
  cz: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="9" fill="#fff"/><rect y="9" width="24" height="9" fill="#D7141A"/><polygon points="0,0 12,9 0,18" fill="#11457E"/></svg>',
  se: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#006AA7"/><rect x="7" width="4" height="18" fill="#FECC00"/><rect y="7" width="24" height="4" fill="#FECC00"/></svg>',
  fi: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#fff"/><rect x="7" width="4" height="18" fill="#003580"/><rect y="7" width="24" height="4" fill="#003580"/></svg>',
  ro: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="8" height="18" fill="#002B7F"/><rect x="8" width="8" height="18" fill="#FCD116"/><rect x="16" width="8" height="18" fill="#CE1126"/></svg>',
  hu: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="6" fill="#CD2A3E"/><rect y="6" width="24" height="6" fill="#fff"/><rect y="12" width="24" height="6" fill="#436F4D"/></svg>',
  gr: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="2" fill="#0D5EAF"/><rect y="2" width="24" height="2" fill="#fff"/><rect y="4" width="24" height="2" fill="#0D5EAF"/><rect y="6" width="24" height="2" fill="#fff"/><rect y="8" width="24" height="2" fill="#0D5EAF"/><rect y="10" width="24" height="8" fill="#0D5EAF"/><rect x="0" y="10" width="8" height="8" fill="#fff"/><rect x="2.5" y="11.5" width="1.5" height="5" fill="#0D5EAF"/><rect x="0" y="13" width="5" height="1.5" fill="#0D5EAF"/></svg>',
  bg: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="6" fill="#fff"/><rect y="6" width="24" height="6" fill="#00966E"/><rect y="12" width="24" height="6" fill="#D62612"/></svg>',
  dk: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#C60C30"/><rect x="8" width="3" height="18" fill="#fff"/><rect y="7" width="24" height="3" fill="#fff"/></svg>',
  sk: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="6" fill="#fff"/><rect y="6" width="24" height="6" fill="#0B4EA2"/><rect y="12" width="24" height="6" fill="#EE1C25"/></svg>',
  lt: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="6" fill="#FDB913"/><rect y="6" width="24" height="6" fill="#006A44"/><rect y="12" width="24" height="6" fill="#C1272D"/></svg>',
  lv: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="8" fill="#9E3039"/><rect y="8" width="24" height="2" fill="#fff"/><rect y="10" width="24" height="8" fill="#9E3039"/></svg>',
  ee: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="6" fill="#0072CE"/><rect y="6" width="24" height="6" fill="#000"/><rect y="12" width="24" height="6" fill="#fff"/></svg>',
  si: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="6" fill="#fff"/><rect y="6" width="24" height="6" fill="#005CE5"/><rect y="12" width="24" height="6" fill="#E60000"/></svg>',
  hr: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="6" fill="#FF0000"/><rect y="6" width="24" height="6" fill="#fff"/><rect y="12" width="24" height="6" fill="#171796"/></svg>',
  no: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#EF2B2D"/><rect x="7" width="4" height="18" fill="#fff"/><rect y="7" width="24" height="4" fill="#fff"/><rect x="8" width="2" height="18" fill="#002868"/><rect y="8" width="24" height="2" fill="#002868"/></svg>',
  tr: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#E30A17"/><circle cx="9" cy="9" r="4.5" fill="#fff"/><circle cx="10" cy="9" r="3.75" fill="#E30A17"/><polygon points="16,7 17.5,8.5 16.5,10 18,9 15.5,9" fill="#fff"/></svg>',
  by: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="12" fill="#C8212D"/><rect y="12" width="24" height="6" fill="#007C30"/><rect x="0" y="0" width="4" height="18" fill="#fff"/></svg>',
  // Asia
  cn: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#DE2910"/><text x="4" y="14" font-size="12" fill="#FFDE00">★</text></svg>',
  jp: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#fff"/><circle cx="12" cy="9" r="5" fill="#BC002D"/></svg>',
  kr: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#fff"/><circle cx="8" cy="9" r="5" fill="#C60C30"/><circle cx="8" cy="9" r="3.5" fill="#003478"/></svg>',
  in: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="5" fill="#FF9933"/><rect y="5" width="24" height="8" fill="#fff"/><rect y="13" width="24" height="5" fill="#138808"/><circle cx="12" cy="9" r="2.5" fill="#000080"/></svg>',
  sa: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#006C35"/><text x="5" y="14" font-size="11" fill="#fff">ﷲ</text></svg>',
  th: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="2.5" fill="#ED1C24"/><rect y="2.5" width="24" height="2.5" fill="#fff"/><rect y="5" width="24" height="8" fill="#241D4F"/><rect y="13" width="24" height="2.5" fill="#fff"/><rect y="15.5" width="24" height="2.5" fill="#ED1C24"/></svg>',
  vn: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#DA251D"/><text x="8" y="14" font-size="10" fill="#FFD700">★</text></svg>',
  id: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="9" fill="#FF0000"/><rect y="9" width="24" height="9" fill="#fff"/></svg>',
  my: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="1.1" fill="#CC0000"/><rect y="1.1" width="24" height="1.1" fill="#fff"/><rect y="2.2" width="24" height="1.1" fill="#CC0000"/><rect y="3.3" width="24" height="1.1" fill="#fff"/><rect y="4.4" width="24" height="1.1" fill="#CC0000"/><rect y="5.5" width="24" height="1.1" fill="#fff"/><rect y="6.6" width="24" height="1.1" fill="#CC0000"/><rect y="7.7" width="24" height="1.1" fill="#fff"/><rect y="8.8" width="24" height="1.1" fill="#CC0000"/><rect y="9.9" width="24" height="1.1" fill="#fff"/><rect y="11" width="24" height="1.1" fill="#CC0000"/><rect y="12.1" width="24" height="1.1" fill="#fff"/><rect y="13.2" width="24" height="1.1" fill="#CC0000"/><rect y="14.3" width="24" height="1.1" fill="#fff"/><rect y="15.5" width="24" height="2.5" fill="#CC0000"/><rect width="10" height="9" fill="#000066"/><text x="2.5" y="10" font-size="7" fill="#FFD700">★</text></svg>',
  ir: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="6" fill="#239F40"/><rect y="6" width="24" height="6" fill="#fff"/><rect y="12" width="24" height="6" fill="#DA0000"/></svg>',
  il: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="3" fill="#fff"/><rect y="3" width="24" height="12" fill="#0038B8"/><rect y="15" width="24" height="3" fill="#fff"/></svg>',
  bd: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#006A4E"/><circle cx="10" cy="9" r="4.5" fill="#F42A41"/></svg>',
  pk: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="5" fill="#fff"/><rect y="5" width="24" height="13" fill="#01411C"/><text x="13" y="13" font-size="9" fill="#fff">★</text></svg>',
  // Africa
  ke: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="4.5" fill="#000"/><rect y="4.5" width="24" height="9" fill="#BB0000"/><rect y="13.5" width="24" height="4.5" fill="#006600"/></svg>',
  et_af: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="4.5" fill="#078930"/><rect y="4.5" width="24" height="9" fill="#FCDD09"/><rect y="13.5" width="24" height="4.5" fill="#DA121A"/></svg>',
  za: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="4.5" fill="#DE3831"/><rect y="4.5" width="24" height="9" fill="#fff"/><rect y="13.5" width="24" height="4.5" fill="#007A4D"/><polygon points="0,0 10,9 0,18" fill="#002395"/></svg>',
  ng: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="8" height="18" fill="#008751"/><rect x="8" width="8" height="18" fill="#fff"/><rect x="16" width="8" height="18" fill="#008751"/></svg>',
  // Americas
  br: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18"><rect width="24" height="18" fill="#009B3A"/><polygon points="12,2 20,9 12,16 4,9" fill="#FEDF00"/><circle cx="11" cy="9" r="4" fill="#002776"/></svg>',
};

export default function FlagIcon({ code }: Props) {
  const key = MAP[code] || 'us';
  return (
    <span
      dangerouslySetInnerHTML={{ __html: SVG[key] || SVG.us }}
      style={{
        display: 'inline-block', width: 18, height: 13, borderRadius: 2,
        verticalAlign: 'middle', marginRight: 5, flexShrink: 0,
        overflow: 'hidden',
      }}
    />
  );
}
