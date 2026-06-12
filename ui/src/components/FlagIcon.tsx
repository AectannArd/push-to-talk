interface Props { code: string }

const MAP: Record<string, string> = {
  'en-US': 'us', 'ru-RU': 'ru', 'de-DE': 'de', 'fr-FR': 'fr',
  'es-ES': 'es', 'it-IT': 'it', 'pt-PT': 'pt', 'pl-PL': 'pl',
  'uk-UA': 'ua', 'nl-NL': 'nl', 'cs-CZ': 'cz', 'sv-SE': 'se',
  'fi-FI': 'fi', 'ro-RO': 'ro', 'hu-HU': 'hu', 'el-GR': 'gr',
  'bg-BG': 'bg', 'da-DK': 'dk', 'sk-SK': 'sk', 'lt-LT': 'lt',
  'lv-LV': 'lv', 'et-EE': 'ee', 'sl-SI': 'si', 'hr-HR': 'hr',
  'no-NO': 'no', 'tr-TR': 'tr', 'be-BY': 'by',
  'zh-CN': 'cn', 'ja-JP': 'jp', 'ko-KR': 'kr', 'hi-IN': 'in',
  'ar-SA': 'sa', 'th-TH': 'th', 'vi-VN': 'vn', 'id-ID': 'id',
  'ms-MY': 'my', 'fa-IR': 'ir', 'he-IL': 'il', 'bn-BD': 'bd',
  'ur-PK': 'pk', 'ta-IN': 'in', 'te-IN': 'in', 'sw-KE': 'ke',
  'am-ET': 'et_af', 'zu-ZA': 'za', 'af-ZA': 'za', 'ha-NG': 'ng',
  'yo-NG': 'ng', 'pt-BR': 'br',
};

type FlagFn = () => string;
const svg = (body: string) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 18" style="display:block">${body}</svg>`;
const rect = (x: string, y: string, w: string, h: string, fill: string) =>
  `<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="${fill}"/>`;

const flags: Record<string, FlagFn> = {
  us: () => svg(rect('0','0','24','18','#B22234') +
    '<g stroke="#fff" stroke-width="1.6">' +
    '<line x1="0" y1="2.25" x2="24" y2="2.25"/><line x1="0" y1="6.75" x2="24" y2="6.75"/>' +
    '<line x1="0" y1="11.25" x2="24" y2="11.25"/><line x1="0" y1="15.75" x2="24" y2="15.75"/></g>' +
    rect('0','0','10','9.75','#3C3B6E')),
  ru: () => svg(rect('0','0','24','6','#fff')+rect('0','6','24','6','#0039A6')+rect('0','12','24','6','#D52B1E')),
  de: () => svg(rect('0','0','24','6','#000')+rect('0','6','24','6','#DD0000')+rect('0','12','24','6','#FFCE00')),
  fr: () => svg(rect('0','0','8','18','#002395')+rect('8','0','8','18','#fff')+rect('16','0','8','18','#ED2939')),
  es: () => svg(rect('0','0','24','4.5','#AA151B')+rect('0','4.5','24','9','#F1BF00')+rect('0','13.5','24','4.5','#AA151B')),
  it: () => svg(rect('0','0','8','18','#009246')+rect('8','0','8','18','#fff')+rect('16','0','8','18','#CE2B37')),
  pt: () => svg(rect('0','0','8','18','#006600')+rect('8','0','16','18','#FF0000')),
  pl: () => svg(rect('0','0','24','9','#fff')+rect('0','9','24','9','#DC143C')),
  ua: () => svg(rect('0','0','24','9','#0057B7')+rect('0','9','24','9','#FFD700')),
  nl: () => svg(rect('0','0','24','6','#AE1C28')+rect('0','6','24','6','#fff')+rect('0','12','24','6','#21468B')),
  cz: () => svg(rect('0','0','24','9','#fff')+rect('0','9','24','9','#D7141A')+'<polygon points="0,0 12,9 0,18" fill="#11457E"/>'),
  se: () => svg(rect('0','0','24','18','#006AA7')+rect('7','0','4','18','#FECC00')+rect('0','7','24','4','#FECC00')),
  fi: () => svg(rect('0','0','24','18','#fff')+rect('7','0','4','18','#003580')+rect('0','7','24','4','#003580')),
  ro: () => svg(rect('0','0','8','18','#002B7F')+rect('8','0','8','18','#FCD116')+rect('16','0','8','18','#CE1126')),
  hu: () => svg(rect('0','0','24','6','#CD2A3E')+rect('0','6','24','6','#fff')+rect('0','12','24','6','#436F4D')),
  gr: () => svg(rect('0','0','24','2','#0D5EAF')+rect('0','2','24','2','#fff')+rect('0','4','24','2','#0D5EAF')+rect('0','6','24','2','#fff')+rect('0','8','24','10','#0D5EAF')+rect('0','10','8','8','#fff')+rect('2.5','11.5','1.5','5','#0D5EAF')+rect('0','13','5','1.5','#0D5EAF')),
  bg: () => svg(rect('0','0','24','6','#fff')+rect('0','6','24','6','#00966E')+rect('0','12','24','6','#D62612')),
  dk: () => svg(rect('0','0','24','18','#C60C30')+rect('8','0','3','18','#fff')+rect('0','7','24','3','#fff')),
  sk: () => svg(rect('0','0','24','6','#fff')+rect('0','6','24','6','#0B4EA2')+rect('0','12','24','6','#EE1C25')),
  lt: () => svg(rect('0','0','24','6','#FDB913')+rect('0','6','24','6','#006A44')+rect('0','12','24','6','#C1272D')),
  lv: () => svg(rect('0','0','24','8','#9E3039')+rect('0','8','24','2','#fff')+rect('0','10','24','8','#9E3039')),
  ee: () => svg(rect('0','0','24','6','#0072CE')+rect('0','6','24','6','#000')+rect('0','12','24','6','#fff')),
  si: () => svg(rect('0','0','24','6','#fff')+rect('0','6','24','6','#005CE5')+rect('0','12','24','6','#E60000')),
  hr: () => svg(rect('0','0','24','6','#FF0000')+rect('0','6','24','6','#fff')+rect('0','12','24','6','#171796')),
  no: () => svg(rect('0','0','24','18','#EF2B2D')+rect('7','0','4','18','#fff')+rect('0','7','24','4','#fff')+rect('8','0','2','18','#002868')+rect('0','8','24','2','#002868')),
  tr: () => svg(rect('0','0','24','18','#E30A17')+'<circle cx="9" cy="9" r="4.5" fill="#fff"/><circle cx="10" cy="9" r="3.75" fill="#E30A17"/><polygon points="16,7 17.5,8.5 16.5,10 18,9 15.5,9" fill="#fff"/>'),
  by: () => svg(rect('0','0','24','12','#C8212D')+rect('0','12','24','6','#007C30')+rect('0','0','4','18','#fff')),
  cn: () => svg(rect('0','0','24','18','#DE2910')+'<text x="4" y="14" font-size="12" fill="#FFDE00" style="fill:#FFDE00">★</text>'),
  jp: () => svg(rect('0','0','24','18','#fff')+'<circle cx="12" cy="9" r="5" fill="#BC002D"/>'),
  kr: () => svg(rect('0','0','24','18','#fff')+'<circle cx="8" cy="9" r="5" fill="#C60C30"/><circle cx="8" cy="9" r="3.5" fill="#003478"/>'),
  in: () => svg(rect('0','0','24','5','#FF9933')+rect('0','5','24','8','#fff')+rect('0','13','24','5','#138808')+'<circle cx="12" cy="9" r="2.5" fill="#000080"/>'),
  sa: () => svg(rect('0','0','24','18','#006C35')+'<text x="5" y="14" font-size="11" fill="#fff" style="fill:#fff">ﷲ</text>'),
  th: () => svg(rect('0','0','24','2.5','#ED1C24')+rect('0','2.5','24','2.5','#fff')+rect('0','5','24','8','#241D4F')+rect('0','13','24','2.5','#fff')+rect('0','15.5','24','2.5','#ED1C24')),
  vn: () => svg(rect('0','0','24','18','#DA251D')+'<text x="8" y="14" font-size="10" fill="#FFD700" style="fill:#FFD700">★</text>'),
  id: () => svg(rect('0','0','24','9','#FF0000')+rect('0','9','24','9','#fff')),
  my: () => svg(rect('0','0','24','1.1','#CC0000')+rect('0','1.1','24','1.1','#fff')+rect('0','2.2','24','1.1','#CC0000')+rect('0','3.3','24','1.1','#fff')+rect('0','4.4','24','1.1','#CC0000')+rect('0','5.5','24','1.1','#fff')+rect('0','6.6','24','1.1','#CC0000')+rect('0','7.7','24','1.1','#fff')+rect('0','8.8','24','1.1','#CC0000')+rect('0','9.9','24','1.1','#fff')+rect('0','11','24','1.1','#CC0000')+rect('0','12.1','24','1.1','#fff')+rect('0','13.2','24','1.1','#CC0000')+rect('0','14.3','24','1.1','#fff')+rect('0','15.5','24','2.5','#CC0000')+rect('0','0','10','9','#000066')+'<text x="2.5" y="10" font-size="7" fill="#FFD700" style="fill:#FFD700">★</text>'),
  ir: () => svg(rect('0','0','24','6','#239F40')+rect('0','6','24','6','#fff')+rect('0','12','24','6','#DA0000')),
  il: () => svg(rect('0','0','24','3','#fff')+rect('0','3','24','12','#0038B8')+rect('0','15','24','3','#fff')),
  bd: () => svg(rect('0','0','24','18','#006A4E')+'<circle cx="10" cy="9" r="4.5" fill="#F42A41"/>'),
  pk: () => svg(rect('0','0','24','5','#fff')+rect('0','5','24','13','#01411C')+'<text x="13" y="13" font-size="9" fill="#fff" style="fill:#fff">★</text>'),
  ke: () => svg(rect('0','0','24','4.5','#000')+rect('0','4.5','24','9','#BB0000')+rect('0','13.5','24','4.5','#006600')),
  et_af: () => svg(rect('0','0','24','4.5','#078930')+rect('0','4.5','24','9','#FCDD09')+rect('0','13.5','24','4.5','#DA121A')),
  za: () => svg(rect('0','0','24','4.5','#DE3831')+rect('0','4.5','24','9','#fff')+rect('0','13.5','24','4.5','#007A4D')+'<polygon points="0,0 10,9 0,18" fill="#002395"/>'),
  ng: () => svg(rect('0','0','8','18','#008751')+rect('8','0','8','18','#fff')+rect('16','0','8','18','#008751')),
  br: () => svg(rect('0','0','24','18','#009B3A')+'<polygon points="12,2 20,9 12,16 4,9" fill="#FEDF00"/><circle cx="11" cy="9" r="4" fill="#002776"/>'),
};

const cache: Record<string, string> = {};

export default function FlagIcon({ code }: Props) {
  const key = MAP[code] || 'us';
  if (!cache[key]) cache[key] = (flags[key] || flags.us)();
  return (
    <span
      dangerouslySetInnerHTML={{ __html: cache[key] }}
      style={{
        display: 'inline-block', width: 18, height: 13, borderRadius: 2,
        verticalAlign: 'middle', marginRight: 5, flexShrink: 0, overflow: 'hidden',
      }}
    />
  );
}
