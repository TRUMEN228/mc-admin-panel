# Генерация SVG-иконок приложений (панель, коллектор, мобильное). PNG рендерится через Edge headless.
import pathlib
ROOT = pathlib.Path(__file__).parent

def icon(g1, g2, badge=''):
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<defs>
  <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{g1}"/><stop offset="1" stop-color="{g2}"/></linearGradient>
  <radialGradient id="hl" cx=".3" cy=".18" r=".8"><stop offset="0" stop-color="#fff" stop-opacity=".38"/><stop offset=".55" stop-color="#fff" stop-opacity="0"/></radialGradient>
  <linearGradient id="top" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#f0fdfa"/></linearGradient>
  <filter id="sh" x="-20%" y="-20%" width="140%" height="150%"><feDropShadow dx="0" dy="28" stdDeviation="30" flood-color="#000" flood-opacity=".28"/></filter>
</defs>
<rect x="64" y="64" width="896" height="896" rx="220" fill="url(#bg)"/>
<rect x="64" y="64" width="896" height="896" rx="220" fill="url(#hl)"/>
<rect x="66" y="66" width="892" height="892" rx="218" fill="none" stroke="#fff" stroke-opacity=".22" stroke-width="4"/>
<g filter="url(#sh)">
  <path d="M512 236 L760 380 L512 524 L264 380 Z" fill="url(#top)"/>
  <path d="M264 380 L512 524 L512 808 L264 664 Z" fill="#fff" fill-opacity=".78"/>
  <path d="M760 380 L512 524 L512 808 L760 664 Z" fill="#fff" fill-opacity=".55"/>
  <path d="M352 560 l64 37 M352 620 l64 37" stroke="{g2}" stroke-opacity=".9" stroke-width="22" stroke-linecap="round"/>
  <circle cx="634" cy="606" r="20" fill="{g1}"/>
</g>
{badge}
</svg>'''

PHONE = '''<g filter="url(#sh)"><rect x="640" y="560" width="230" height="350" rx="48" fill="#0b1220"/><rect x="662" y="590" width="186" height="270" rx="26" fill="#22d3ee"/><rect x="725" y="880" width="60" height="10" rx="5" fill="#fff" fill-opacity=".6"/></g>'''
PLUS = '''<g filter="url(#sh)"><circle cx="770" cy="770" r="130" fill="#0b0b10"/><path d="M770 705 v130 M705 770 h130" stroke="#fff" stroke-width="34" stroke-linecap="round"/></g>'''

for name, g1, g2, badge in [('panel', '#10b981', '#06b6d4', ''), ('mobile', '#10b981', '#06b6d4', PHONE), ('collector', '#a855f7', '#f97316', PLUS)]:
    (ROOT / f'{name}.svg').write_text(icon(g1, g2, badge), encoding='utf-8')
    (ROOT / f'{name}.html').write_text(f'<!doctype html><html><body style="margin:0;background:transparent">{icon(g1, g2, badge)}</body></html>', encoding='utf-8')
    print('ok', name)
