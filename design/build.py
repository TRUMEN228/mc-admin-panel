# Генератор демо-вариантов дизайна: одна раскладка, три стиля.  python design/build.py
import pathlib

ROOT = pathlib.Path(__file__).parent

ICONS = {
    'home': '<path d="M3 11l9-8 9 8v9a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z"/>',
    'server': '<rect x="3" y="4" width="18" height="7" rx="2"/><rect x="3" y="13" width="18" height="7" rx="2"/><path d="M7 7.5h.01M7 16.5h.01"/>',
    'package': '<path d="M21 8l-9-5-9 5 9 5 9-5zM3 8v8l9 5 9-5V8M12 13v8"/>',
    'archive': '<rect x="3" y="4" width="18" height="5" rx="1"/><path d="M5 9v10a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V9M10 13h4"/>',
    'settings': '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/>',
    'search': '<circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3"/>',
    'play': '<path d="M7 4l13 8-13 8z"/>',
    'stop': '<rect x="6" y="6" width="12" height="12" rx="2"/>',
    'restart': '<path d="M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5"/>',
    'terminal': '<path d="M4 17l6-5-6-5M12 19h8"/>',
    'users': '<circle cx="9" cy="8" r="4"/><path d="M2 21a7 7 0 0 1 14 0M16 4a4 4 0 0 1 0 8M22 21a7 7 0 0 0-4-6.3"/>',
    'cpu': '<rect x="5" y="5" width="14" height="14" rx="2"/><rect x="9" y="9" width="6" height="6"/><path d="M9 2v3M15 2v3M9 19v3M15 19v3M2 9h3M2 15h3M19 9h3M19 15h3"/>',
    'memory': '<rect x="2" y="7" width="20" height="10" rx="2"/><path d="M6 11v2M10 11v2M14 11v2M18 11v2"/>',
    'plus': '<path d="M12 5v14M5 12h14"/>',
    'bell': '<path d="M6 8a6 6 0 1 1 12 0c0 7 3 9 3 9H3s3-2 3-9M10 21a2 2 0 0 0 4 0"/>',
    'alert': '<path d="M12 3l10 18H2zM12 10v4M12 17.5h.01"/>',
    'check': '<path d="M5 12l5 5 9-10"/>',
    'phone': '<rect x="6" y="2" width="12" height="20" rx="3"/><path d="M11 18h2"/>',
    'clock': '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
    'more': '<circle cx="5" cy="12" r="1.5"/><circle cx="12" cy="12" r="1.5"/><circle cx="19" cy="12" r="1.5"/>',
    'chevron': '<path d="M9 6l6 6-6 6"/>',
    'globe': '<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"/>',
}

def ic(n, cls=''):
    return f'<svg class="ic {cls}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">{ICONS[n]}</svg>'

SERVERS = [
    ('Survival', 'Fabric 1.21.11', 'running', 23.4, '2,1', '6', 7, 20, '3 д 4 ч', 'a'),
    ('Creative', 'Paper 1.21.4', 'running', 6.2, '1,4', '4', 2, 20, '18 ч', 'b'),
    ('Modpack RPG', 'NeoForge 1.21.1', 'starting', 41.0, '3,8', '8', 0, 10, '—', 'c'),
    ('Мини-игры', 'Paper 1.20.4', 'stopped', 0, '—', '4', 0, 30, '—', 'd'),
    ('Тест 1.19', 'Vanilla 1.19.4', 'stopped', 0, '—', '2', 0, 10, '—', 'e'),
]
STATUS = {'running': 'Работает', 'starting': 'Запуск…', 'stopped': 'Остановлен'}
FEED = [
    ('ok', 'users', 'Steve_228 зашёл на Survival', '2 мин'),
    ('info', 'archive', 'Бэкап Creative создан · 412 МБ', '14 мин'),
    ('warn', 'alert', 'Modpack RPG: долгий запуск (43 мода)', '20 мин'),
    ('ok', 'package', 'Обновлено 5 модов на Survival', '1 ч'),
    ('err', 'alert', 'Мини-игры аварийно завершился', '3 ч'),
]

def spark(seed, n=28):
    import math
    pts = [50 + 30 * math.sin(i / 3 + seed) + 12 * math.sin(i * 1.7 + seed * 2) for i in range(n)]
    w = 100 / (n - 1)
    d = 'M' + ' L'.join(f'{i*w:.1f},{100-p:.1f}' for i, p in enumerate(pts))
    return f'<svg class="spark" viewBox="0 0 100 100" preserveAspectRatio="none"><path class="area" d="{d} L100,100 L0,100 Z"/><path class="line" d="{d}"/></svg>'

def card(s, i):
    name, ver, st, cpu, mem, ram, pl, mx, up, seed = s
    on = st != 'stopped'
    acts = (f'<button class="btn ghost sm">{ic("restart")}</button><button class="btn ghost sm">{ic("stop")}</button>' if on
            else f'<button class="btn primary sm">{ic("play")}Запустить</button>')
    meters = f'''<div class="meters">
        <div><span>{ic("cpu")}CPU</span><b>{str(cpu).replace(".", ",")}%</b><i class="bar"><em style="width:{cpu}%"></em></i></div>
        <div><span>{ic("memory")}RAM</span><b>{mem} <small>/ {ram} ГБ</small></b><i class="bar"><em style="width:{(float(mem.replace(",", ".")) / float(ram) * 100) if on else 0:.0f}%"></em></i></div>
        <div><span>{ic("users")}Игроки</span><b>{pl} <small>/ {mx}</small></b><i class="bar"><em style="width:{pl / mx * 100:.0f}%"></em></i></div>
      </div>''' if on else '<div class="off-note">Сервер остановлен · последний запуск вчера</div>'
    return f'''<article class="srv {st}" style="--i:{i}">
      <div class="srv-art art-{seed}"></div>
      <header><span class="dot"></span><div class="nm"><h3>{name}</h3><p>{ver} · :2556{i}</p></div><span class="pill">{STATUS[st]}</span></header>
      {meters}
      <footer><span class="up">{ic("clock")}{up}</span><div class="acts">{acts}<button class="btn ghost sm">{ic("terminal")}</button></div></footer>
    </article>'''

def page(style, title, css, fonts, extra_js=''):
    servers = ''.join(card(s, i) for i, s in enumerate(SERVERS))
    feed = ''.join(f'<li class="{k}" style="--i:{i}"><span class="fi">{ic(icn)}</span><div><p>{t}</p><time>{tm} назад</time></div></li>' for i, (k, icn, t, tm) in enumerate(FEED))
    nav = ''.join(f'<a class="{"on" if i == 0 else ""}" title="{t}">{ic(n)}<span>{t}</span></a>' for i, (n, t) in enumerate([('home', 'Обзор'), ('server', 'Серверы'), ('package', 'Моды'), ('archive', 'Бэкапы'), ('globe', 'Сеть'), ('settings', 'Настройки')]))
    phone_list = ''.join(f'''<div class="p-srv {s[2]}" style="--i:{i}"><span class="dot"></span><div><b>{s[0]}</b><small>{s[1]}</small></div><span class="p-meta">{(str(s[3]).replace(".", ",") + "%") if s[2] != "stopped" else "выкл"}</span></div>''' for i, s in enumerate(SERVERS[:4]))
    return f'''<!doctype html>
<html lang="ru"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title}</title>
<link rel="preconnect" href="https://fonts.googleapis.com"><link href="https://fonts.googleapis.com/css2?{fonts}&display=swap" rel="stylesheet">
<style>
*{{box-sizing:border-box;margin:0;padding:0}} button{{font:inherit;color:inherit;border:0;background:none;cursor:pointer}}
.ic{{width:18px;height:18px;flex-shrink:0}}
body{{min-height:100vh;overflow-x:hidden}}
.stage{{display:grid;grid-template-columns:minmax(0,1fr) 340px;gap:32px;padding:28px;max-width:1680px;margin:0 auto;align-items:start}}
.cap{{grid-column:1/-1;display:flex;align-items:baseline;gap:14px;flex-wrap:wrap}}
.cap h1{{font-size:15px;letter-spacing:.08em;text-transform:uppercase}} .cap p{{font-size:13px;opacity:.65}}
.app{{display:grid;grid-template-columns:76px 1fr;height:820px;overflow:hidden;position:relative}}
.rail{{display:flex;flex-direction:column;align-items:center;gap:6px;padding:16px 0}}
.rail .logo{{width:42px;height:42px;display:grid;place-items:center;margin-bottom:14px}}
.rail a{{width:52px;height:52px;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:3px;font-size:10px;cursor:pointer;position:relative}}
.rail a .ic{{width:20px;height:20px}} .rail .grow{{flex:1}}
.main{{display:flex;flex-direction:column;min-width:0;min-height:0}}
.top{{display:flex;align-items:center;gap:14px;padding:16px 24px}}
.top h2{{font-size:22px;flex:1}}
.cmd{{display:flex;align-items:center;gap:10px;width:340px;height:40px;padding:0 14px;font-size:13.5px}}
.cmd kbd{{margin-left:auto;font:11px inherit;padding:2px 6px}}
.iconbtn{{width:40px;height:40px;display:grid;place-items:center;position:relative}}
.iconbtn .badge{{position:absolute;top:8px;right:9px;width:8px;height:8px;border-radius:50%}}
.content{{flex:1;display:grid;grid-template-columns:minmax(0,1fr) 300px;gap:20px;padding:4px 24px 24px;min-height:0}}
.col{{display:flex;flex-direction:column;gap:20px;min-height:0;overflow:auto;scrollbar-width:none}}
.kpis{{display:grid;grid-template-columns:repeat(4,1fr);gap:14px}}
.kpi{{padding:16px 16px 0;position:relative;overflow:hidden;animation:rise .6s var(--ease) both;animation-delay:calc(var(--i)*70ms)}}
.kpi .k{{display:flex;align-items:center;gap:7px;font-size:12.5px}}
.kpi .v{{font-size:30px;margin:6px 0 2px;font-variant-numeric:tabular-nums}} .kpi .v small{{font-size:14px;margin-left:3px}}
.kpi .d{{font-size:12px}}
.spark{{display:block;width:calc(100% + 32px);height:46px;margin:8px -16px 0}}
.spark .line{{fill:none;stroke-width:2;vector-effect:non-scaling-stroke;stroke-dasharray:400;stroke-dashoffset:400;animation:draw 1.6s var(--ease) .3s forwards}}
.sec-h{{display:flex;align-items:center;gap:10px}} .sec-h h4{{font-size:15px;flex:1}}
.seg{{display:flex;padding:3px;gap:2px}} .seg button{{padding:6px 12px;font-size:12.5px}}
.grid{{display:grid;grid-template-columns:repeat(auto-fill,minmax(260px,1fr));gap:14px}}
.srv{{position:relative;overflow:hidden;padding:16px;display:flex;flex-direction:column;gap:14px;animation:rise .6s var(--ease) both;animation-delay:calc(200ms + var(--i)*80ms);transition:transform .35s var(--ease),box-shadow .35s var(--ease),border-color .35s}}
.srv:hover{{transform:translateY(-4px)}}
.srv-art{{position:absolute;inset:0 0 auto;height:64px;opacity:.55;pointer-events:none}}
.srv header{{display:flex;align-items:center;gap:10px;position:relative}}
.srv .nm{{flex:1;min-width:0}} .srv h3{{font-size:16px}} .srv .nm p{{font-size:12px;margin-top:2px}}
.dot{{width:10px;height:10px;border-radius:50%;flex-shrink:0;position:relative}}
.running .dot::after,.starting .dot::after{{content:"";position:absolute;inset:-4px;border-radius:50%;border:2px solid currentColor;animation:ping 1.8s ease-out infinite}}
.pill{{font-size:11.5px;padding:4px 9px;white-space:nowrap}}
.meters{{display:grid;gap:9px}} .meters>div{{display:grid;grid-template-columns:1fr auto;align-items:center;gap:3px 8px;font-size:12px}}
.meters span{{display:flex;align-items:center;gap:6px}} .meters span .ic{{width:14px;height:14px}}
.meters b{{font-size:13px;font-variant-numeric:tabular-nums}} .meters b small{{font-weight:400;opacity:.6}}
.bar{{grid-column:1/-1;display:block;height:5px;overflow:hidden}} .bar em{{display:block;height:100%;transform-origin:left;animation:grow 1.2s var(--ease) .5s both}}
.off-note{{font-size:12.5px;padding:18px 0 24px;text-align:center}}
.srv footer{{display:flex;align-items:center;gap:8px}} .up{{display:flex;align-items:center;gap:5px;font-size:12px;flex:1}} .up .ic{{width:14px;height:14px}}
.acts{{display:flex;gap:6px}}
.btn{{display:inline-flex;align-items:center;justify-content:center;gap:7px;height:36px;padding:0 14px;font-size:13.5px;transition:all .2s var(--ease)}}
.btn.sm{{height:32px;padding:0 10px;font-size:12.5px}} .btn.sm .ic{{width:16px;height:16px}}
.btn:active{{transform:scale(.96)}}
.add{{display:grid;place-items:center;align-content:center;gap:8px;min-height:180px;font-size:13.5px;animation:rise .6s var(--ease) both;animation-delay:700ms;cursor:pointer;transition:all .3s var(--ease)}}
.add .ic{{width:28px;height:28px}}
.side{{display:flex;flex-direction:column;gap:16px;min-height:0}}
.panel{{padding:16px;animation:rise .6s var(--ease) both;animation-delay:300ms}}
.panel h4{{font-size:14px;margin-bottom:12px;display:flex;align-items:center;gap:8px}}
.feed{{list-style:none;display:flex;flex-direction:column;gap:12px}}
.feed li{{display:flex;gap:10px;font-size:12.5px;animation:slide .5s var(--ease) both;animation-delay:calc(500ms + var(--i)*90ms)}}
.feed .fi{{width:30px;height:30px;display:grid;place-items:center;flex-shrink:0}} .feed .fi .ic{{width:15px;height:15px}}
.feed time{{font-size:11.5px;opacity:.55}}
.sys .row{{display:flex;justify-content:space-between;font-size:12.5px;margin-bottom:6px}}
.sys .bar{{height:7px;margin-bottom:14px}}
.phone{{width:340px;height:700px;padding:12px;position:sticky;top:28px;animation:rise .8s var(--ease) both .4s}}
.screen{{height:100%;overflow:hidden;display:flex;flex-direction:column;position:relative}}
.p-top{{padding:30px 18px 10px}} .p-top small{{font-size:12px;display:flex;align-items:center;gap:6px}} .p-top h3{{font-size:24px;margin-top:4px}}
.p-hero{{margin:6px 14px 12px;padding:16px;position:relative;overflow:hidden}}
.p-hero .r{{display:flex;justify-content:space-between;align-items:end}} .p-hero b{{font-size:34px}} .p-hero span{{font-size:12px}}
.p-list{{flex:1;padding:0 14px;display:flex;flex-direction:column;gap:9px;overflow:hidden}}
.p-srv{{display:flex;align-items:center;gap:12px;padding:13px 14px;animation:rise .5s var(--ease) both;animation-delay:calc(700ms + var(--i)*80ms)}}
.p-srv>div{{flex:1;min-width:0}} .p-srv b{{display:block;font-size:14.5px}} .p-srv small{{font-size:12px;opacity:.6}} .p-meta{{font-size:13px;font-variant-numeric:tabular-nums}}
.p-nav{{display:flex;justify-content:space-around;padding:10px 0 16px;margin:10px 12px 0}}
.p-nav a{{display:flex;flex-direction:column;align-items:center;gap:3px;font-size:10.5px;padding:6px 14px}}
.fab{{position:absolute;right:18px;bottom:84px;width:52px;height:52px;display:grid;place-items:center}}
@keyframes rise{{from{{opacity:0;transform:translateY(16px) scale(.98)}}}}
@keyframes slide{{from{{opacity:0;transform:translateX(14px)}}}}
@keyframes draw{{to{{stroke-dashoffset:0}}}}
@keyframes grow{{from{{transform:scaleX(0)}}}}
@keyframes ping{{0%{{transform:scale(.6);opacity:.9}}100%{{transform:scale(1.9);opacity:0}}}}
@media (prefers-reduced-motion:reduce){{*,*::before,*::after{{animation:none!important;transition:none!important}}.spark .line{{stroke-dashoffset:0}}}}
@media (max-width:1250px){{.stage{{grid-template-columns:1fr}}.phone{{position:static;margin:0 auto}}.app{{height:auto}}.content{{grid-template-columns:1fr}}}}
:root{{--ease:cubic-bezier(.2,.8,.2,1)}}
{css}
</style></head>
<body class="{style}">
<div class="bg"></div>
<div class="stage">
  <div class="cap"><h1>{title}</h1><p>Демо стиля. Раскладка: узкая навигация слева, поиск/команды сверху (Ctrl+K), сводка, серверы плиткой, лента событий и ресурсы ПК справа. Справа — мобильное приложение в том же стиле.</p></div>
  <div class="app">
    <nav class="rail"><div class="logo">{ic("server")}</div>{nav}<div class="grow"></div><a title="Телефон">{ic("phone")}<span>Телефон</span></a></nav>
    <div class="main">
      <div class="top"><h2>Обзор</h2><div class="cmd">{ic("search")}Найти сервер, мод, команду…<kbd>Ctrl K</kbd></div>
        <button class="iconbtn">{ic("bell")}<span class="badge"></span></button><button class="btn primary">{ic("plus")}Новый сервер</button></div>
      <div class="content">
        <div class="col">
          <div class="kpis">
            <div class="kpi" style="--i:0"><div class="k">{ic("server")}Серверы</div><div class="v"><span data-count="2">0</span><small>/ 5</small></div><div class="d">работают сейчас</div>{spark(1)}</div>
            <div class="kpi" style="--i:1"><div class="k">{ic("users")}Игроки</div><div class="v"><span data-count="9">0</span><small>онлайн</small></div><div class="d">пик сегодня — 14</div>{spark(2.3)}</div>
            <div class="kpi" style="--i:2"><div class="k">{ic("cpu")}CPU ПК</div><div class="v"><span data-count="34" id="cpu">0</span><small>%</small></div><div class="d">Ryzen 7 · 16 потоков</div>{spark(3.1)}</div>
            <div class="kpi" style="--i:3"><div class="k">{ic("memory")}Память ПК</div><div class="v"><span data-count="18">0</span><small>/ 32 ГБ</small></div><div class="d">серверам выделено 24 ГБ</div>{spark(4.7)}</div>
          </div>
          <div class="sec-h"><h4>Серверы</h4><div class="seg"><button class="on">Все</button><button>Работают</button><button>Остановлены</button></div></div>
          <div class="grid">{servers}<div class="add">{ic("plus")}Создать сервер<small>или импортировать .mrpack</small></div></div>
        </div>
        <div class="side">
          <div class="panel"><h4>{ic("bell")}События</h4><ul class="feed">{feed}</ul></div>
          <div class="panel sys"><h4>{ic("cpu")}Ресурсы ПК</h4>
            <div class="row"><span>Процессор</span><b>34%</b></div><i class="bar"><em style="width:34%"></em></i>
            <div class="row"><span>Память</span><b>18 / 32 ГБ</b></div><i class="bar"><em style="width:56%"></em></i>
            <div class="row"><span>Диск D:</span><b>212 / 931 ГБ</b></div><i class="bar"><em style="width:23%"></em></i></div>
        </div>
      </div>
    </div>
  </div>
  <div class="phone"><div class="screen">
    <div class="p-top"><small><span class="dot" style="width:8px;height:8px"></span>DENIS · подключено</small><h3>Серверы</h3></div>
    <div class="p-hero"><div class="r"><div><span>Игроков онлайн</span><br><b>9</b></div><div style="text-align:right"><span>CPU ПК</span><br><b style="font-size:22px">34%</b></div></div>{spark(5.5)}</div>
    <div class="p-list">{phone_list}</div>
    <button class="fab">{ic("plus")}</button>
    <nav class="p-nav"><a class="on">{ic("server")}Серверы</a><a>{ic("bell")}События</a><a>{ic("settings")}Настройки</a></nav>
  </div></div>
</div>
<script>
// счётчики и «живой» CPU
document.querySelectorAll('[data-count]').forEach((el) => {{
  const to = +el.dataset.count, t0 = performance.now();
  const step = (t) => {{ const k = Math.min(1, (t - t0 - 300) / 1100); if (k > 0) el.textContent = Math.round(to * (1 - Math.pow(1 - k, 3))); if (k < 1) requestAnimationFrame(step); }};
  requestAnimationFrame(step);
}});
setInterval(() => {{ const c = document.getElementById('cpu'); c.textContent = 28 + Math.round(Math.random() * 14); }}, 2500);
// подсветка за курсором
document.querySelectorAll('.srv,.kpi,.panel').forEach((el) => el.addEventListener('pointermove', (e) => {{
  const r = el.getBoundingClientRect(); el.style.setProperty('--mx', (e.clientX - r.left) + 'px'); el.style.setProperty('--my', (e.clientY - r.top) + 'px');
}}));
{extra_js}
</script>
</body></html>'''

# ---------- 1. Glass / Aurora ----------
GLASS = '''
body.glass{font-family:"Inter",system-ui,sans-serif;color:#e8ecf6;background:#070912}
.glass h1,.glass h2,.glass h3,.glass h4,.glass .v,.glass .p-hero b{font-family:"Space Grotesk",sans-serif;font-weight:600}
.glass .bg{position:fixed;inset:0;z-index:-1;overflow:hidden;background:radial-gradient(1200px 700px at 10% -10%,#1b2a6b55,transparent),#070912}
.glass .bg::before,.glass .bg::after{content:"";position:absolute;width:900px;height:900px;border-radius:50%;filter:blur(120px);opacity:.55;animation:float 22s ease-in-out infinite alternate}
.glass .bg::before{background:conic-gradient(from 90deg,#22d3ee,#6366f1,#a855f7,#22d3ee);left:-200px;top:-250px}
.glass .bg::after{background:conic-gradient(from 0deg,#10b981,#22d3ee,#3b82f6,#10b981);right:-300px;bottom:-400px;animation-duration:28s}
@keyframes float{to{transform:translate(160px,120px) rotate(60deg) scale(1.15)}}
.glass .app,.glass .phone{background:rgba(15,19,36,.45);backdrop-filter:blur(28px) saturate(150%);border:1px solid rgba(255,255,255,.09);border-radius:26px;box-shadow:0 30px 80px -20px rgba(0,0,0,.6),inset 0 1px 0 rgba(255,255,255,.08)}
.glass .phone{border-radius:44px;padding:10px}.glass .screen{border-radius:34px;background:rgba(8,10,22,.55)}
.glass .rail{border-right:1px solid rgba(255,255,255,.06)}
.glass .rail .logo{border-radius:14px;background:linear-gradient(135deg,#22d3ee,#6366f1);box-shadow:0 8px 24px -6px #6366f1aa;color:#fff}
.glass .rail a{border-radius:14px;color:#8b93b0;transition:all .25s}
.glass .rail a:hover{color:#fff;background:rgba(255,255,255,.06)}
.glass .rail a.on{color:#fff;background:linear-gradient(135deg,rgba(34,211,238,.22),rgba(99,102,241,.22));box-shadow:inset 0 0 0 1px rgba(129,140,248,.35)}
.glass .cmd,.glass .iconbtn,.glass .seg{background:rgba(255,255,255,.05);border:1px solid rgba(255,255,255,.08);border-radius:12px;color:#8b93b0}
.glass .cmd kbd{background:rgba(255,255,255,.08);border-radius:6px;color:#c7cde0}
.glass .iconbtn .badge{background:#f472b6;box-shadow:0 0 10px #f472b6}
.glass .kpi,.glass .srv,.glass .panel,.glass .p-srv,.glass .p-hero{background:radial-gradient(400px circle at var(--mx,-100px) var(--my,-100px),rgba(129,140,248,.14),transparent 40%),rgba(255,255,255,.04);border:1px solid rgba(255,255,255,.08);border-radius:18px}
.glass .kpi .k,.glass .kpi .d,.glass .srv .nm p,.glass .meters span,.glass .up,.glass .off-note,.glass .p-srv small,.glass .p-top small,.glass .p-hero span{color:#8b93b0}
.glass .kpi:nth-child(1) .line{stroke:#22d3ee}.glass .kpi:nth-child(2) .line{stroke:#a78bfa}.glass .kpi:nth-child(3) .line{stroke:#34d399}.glass .kpi:nth-child(4) .line{stroke:#f472b6}
.glass .spark .area{fill:url(#none);opacity:.12}.glass .kpi:nth-child(1) .area{fill:#22d3ee}.glass .kpi:nth-child(2) .area{fill:#a78bfa}.glass .kpi:nth-child(3) .area{fill:#34d399}.glass .kpi:nth-child(4) .area{fill:#f472b6}
.glass .p-hero .line{stroke:#22d3ee}.glass .p-hero .area{fill:#22d3ee}
.glass .srv:hover{box-shadow:0 20px 50px -18px rgba(99,102,241,.55);border-color:rgba(129,140,248,.35)}
.glass .srv-art{opacity:.35}.glass .art-a{background:linear-gradient(90deg,#22d3ee55,transparent)}.glass .art-b{background:linear-gradient(90deg,#a78bfa55,transparent)}.glass .art-c{background:linear-gradient(90deg,#fbbf2455,transparent)}.glass .art-d,.glass .art-e{background:linear-gradient(90deg,#64748b33,transparent)}
.glass .srv-art{-webkit-mask:linear-gradient(#000,transparent);mask:linear-gradient(#000,transparent)}
.glass .dot{background:#64748b;color:#64748b}.glass .running .dot{background:#34d399;color:#34d399;box-shadow:0 0 12px #34d399}.glass .starting .dot{background:#fbbf24;color:#fbbf24;box-shadow:0 0 12px #fbbf24}
.glass .pill{border-radius:99px;background:rgba(100,116,139,.18);color:#94a3b8}.glass .running .pill{background:rgba(52,211,153,.14);color:#6ee7b7}.glass .starting .pill{background:rgba(251,191,36,.14);color:#fcd34d}
.glass .bar{background:rgba(255,255,255,.07);border-radius:99px}.glass .bar em{border-radius:99px;background:linear-gradient(90deg,#22d3ee,#818cf8)}
.glass .btn{border-radius:11px}.glass .btn.primary{background:linear-gradient(135deg,#22d3ee,#6366f1);color:#fff;box-shadow:0 8px 22px -8px #6366f1;font-weight:600}
.glass .btn.primary:hover{box-shadow:0 10px 30px -6px #6366f1;filter:brightness(1.1)}
.glass .btn.ghost{background:rgba(255,255,255,.06);color:#c7cde0}.glass .btn.ghost:hover{background:rgba(255,255,255,.12)}
.glass .seg button{border-radius:9px}.glass .seg .on{background:rgba(255,255,255,.1);color:#fff}
.glass .add{border:1.5px dashed rgba(255,255,255,.14);border-radius:18px;color:#8b93b0}.glass .add:hover{border-color:#818cf8;color:#c7d2fe;background:rgba(129,140,248,.06)}
.glass .feed .fi{border-radius:10px}.glass .feed .ok .fi{background:rgba(52,211,153,.14);color:#34d399}.glass .feed .info .fi{background:rgba(96,165,250,.14);color:#60a5fa}.glass .feed .warn .fi{background:rgba(251,191,36,.14);color:#fbbf24}.glass .feed .err .fi{background:rgba(248,113,113,.14);color:#f87171}
.glass .p-nav{background:rgba(255,255,255,.05);border:1px solid rgba(255,255,255,.08);border-radius:22px}.glass .p-nav a{color:#8b93b0;border-radius:14px}.glass .p-nav a.on{color:#fff;background:rgba(129,140,248,.2)}
.glass .fab{border-radius:18px;background:linear-gradient(135deg,#22d3ee,#6366f1);color:#fff;box-shadow:0 12px 30px -8px #6366f1}
.glass .cap h1{color:#a5b4fc}
'''

# ---------- 2. Minecraft ----------
MC = '''
body.mc{font-family:"Rubik",system-ui,sans-serif;color:#ecebe4;background:#16130f}
.mc h1,.mc h2,.mc h3,.mc h4,.mc .v,.mc .p-hero b,.mc .pill,.mc .rail a span,.mc kbd{font-family:"Press Start 2P","Rubik",monospace;font-weight:400;letter-spacing:0}
.mc h2{font-size:17px!important}.mc h3{font-size:12px!important;line-height:1.5}.mc h4{font-size:11px!important}.mc .v{font-size:20px!important}.mc .v small{font-size:10px!important}.mc .pill{font-size:8px!important}.mc .rail a span{font-size:6.5px}.mc .p-hero b{font-size:24px!important}.mc .cap h1{font-size:12px!important}
.mc .bg{position:fixed;inset:0;z-index:-1;background:
 linear-gradient(rgba(10,8,6,.82),rgba(10,8,6,.92)),
 repeating-conic-gradient(#3b2a1d 0 25%,#46331f 0 50%) 0 0/32px 32px;}
.mc .bg::after{content:"";position:absolute;inset:0;background:radial-gradient(800px 400px at 50% 0,rgba(94,180,60,.18),transparent 70%)}
.mc .app,.mc .phone{background:#211c16;border:4px solid #0d0b08;box-shadow:inset 4px 4px 0 #3a3128,inset -4px -4px 0 #15110d,0 24px 0 -12px #0007}
.mc .phone{padding:10px;border-radius:0}.mc .screen{background:#1a1611;border:3px solid #0d0b08}
.mc .rail{background:#1a1611;border-right:4px solid #0d0b08}
.mc .rail .logo{background:#5cb83c;border:3px solid #0d0b08;box-shadow:inset 3px 3px 0 #8ee25f,inset -3px -3px 0 #3d7f26;color:#fff;image-rendering:pixelated}
.mc .rail a{color:#a39d8f;border:3px solid transparent}
.mc .rail a:hover{color:#fff;background:#2c251d}
.mc .rail a.on{color:#fff;background:#3d3428;border-color:#0d0b08;box-shadow:inset 3px 3px 0 #574a3a,inset -3px -3px 0 #2a231b}
.mc .cmd,.mc .iconbtn,.mc .seg{background:#0f0d0a;border:3px solid #000;box-shadow:inset 2px 2px 0 #000,inset -2px -2px 0 #3a3128;color:#8f897c}
.mc .cmd kbd{background:#3a3128;color:#d7d0c0;font-size:7px}
.mc .iconbtn .badge{background:#ff5555;border-radius:0;animation:blink 1s steps(2) infinite}
@keyframes blink{50%{opacity:0}}
.mc .kpi,.mc .srv,.mc .panel,.mc .p-srv,.mc .p-hero{background:#2a241c;border:3px solid #0d0b08;box-shadow:inset 3px 3px 0 #3d3428,inset -3px -3px 0 #1c1813}
.mc .kpi .k,.mc .kpi .d,.mc .srv .nm p,.mc .meters span,.mc .up,.mc .off-note,.mc .p-srv small,.mc .p-top small,.mc .p-hero span{color:#a39d8f}
.mc .spark .line{stroke:#5cb83c;stroke-width:3;stroke-linejoin:miter;stroke-linecap:square}.mc .spark .area{fill:#5cb83c;opacity:.12}
.mc .kpi:nth-child(2) .line{stroke:#55c7ff}.mc .kpi:nth-child(2) .area{fill:#55c7ff}.mc .kpi:nth-child(3) .line{stroke:#ffaa00}.mc .kpi:nth-child(3) .area{fill:#ffaa00}.mc .kpi:nth-child(4) .line{stroke:#ff55ff}.mc .kpi:nth-child(4) .area{fill:#ff55ff}
.mc .srv:hover{transform:translateY(-4px);box-shadow:inset 3px 3px 0 #4a3f31,inset -3px -3px 0 #1c1813,0 8px 0 #0d0b08}
.mc .srv-art{height:58px;opacity:1;image-rendering:pixelated;border-bottom:3px solid #0d0b08}
.mc .srv header{margin-top:52px}
.mc .art-a{background:linear-gradient(#5cb83c 0 30%,transparent 30%),repeating-conic-gradient(#7a5536 0 25%,#8a6240 0 50%) 0 0/16px 16px}
.mc .art-b{background:repeating-conic-gradient(#d9d0b7 0 25%,#c4b998 0 50%) 0 0/16px 16px}
.mc .art-c{background:repeating-conic-gradient(#4b2a6b 0 25%,#5c357f 0 50%) 0 0/16px 16px}
.mc .art-d{background:repeating-conic-gradient(#6b6b6b 0 25%,#7b7b7b 0 50%) 0 0/16px 16px}
.mc .art-e{background:repeating-conic-gradient(#b3433b 0 25%,#c24e45 0 50%) 0 0/16px 16px}
.mc .dot{border-radius:0;background:#6b6b6b;color:#6b6b6b}.mc .running .dot{background:#55ff55;color:#55ff55}.mc .starting .dot{background:#ffff55;color:#ffff55}
.mc .running .dot::after,.mc .starting .dot::after{border-radius:0}
.mc .pill{padding:5px 7px;background:#0f0d0a;color:#a39d8f;border:2px solid #000}.mc .running .pill{color:#55ff55}.mc .starting .pill{color:#ffff55}
.mc .bar{height:10px!important;background:#0f0d0a;border:2px solid #000}.mc .bar em{background:repeating-linear-gradient(90deg,#5cb83c 0 8px,#4ea232 8px 10px);animation-timing-function:steps(12)}
.mc .btn{border:3px solid #000;box-shadow:inset 3px 3px 0 #ffffff3a,inset -3px -3px 0 #0005;font-family:"Rubik";font-weight:600}
.mc .btn.primary{background:#5cb83c;color:#fff;text-shadow:2px 2px 0 #2b5c1b}.mc .btn.primary:hover{background:#6fd04b}
.mc .btn.ghost{background:#6f6f6f;color:#fff;text-shadow:1px 1px 0 #333}.mc .btn.ghost:hover{background:#8a8aa8}
.mc .seg button{color:#a39d8f}.mc .seg .on{background:#3d3428;color:#fff}
.mc .add{border:3px dashed #4a3f31;color:#a39d8f}.mc .add:hover{background:#2a241c;color:#5cb83c;border-color:#5cb83c}
.mc .feed .fi{border:2px solid #000}.mc .feed .ok .fi{background:#2d5a1c;color:#8ee25f}.mc .feed .info .fi{background:#1c3d5a;color:#55c7ff}.mc .feed .warn .fi{background:#5a4a1c;color:#ffd255}.mc .feed .err .fi{background:#5a1c1c;color:#ff6b6b}
.mc .sys .bar em{background:repeating-linear-gradient(90deg,#55c7ff 0 8px,#3ea6d8 8px 10px)}
.mc .p-nav{background:#211c16;border:3px solid #0d0b08}.mc .p-nav a{color:#a39d8f}.mc .p-nav a.on{color:#5cb83c}
.mc .fab{background:#5cb83c;border:3px solid #000;box-shadow:inset 3px 3px 0 #8ee25f,inset -3px -3px 0 #3d7f26;color:#fff}
.mc .p-hero .line{stroke:#55c7ff}.mc .p-hero .area{fill:#55c7ff}
.mc .cap h1{color:#5cb83c;text-shadow:3px 3px 0 #000}
.mc .kpi,.mc .srv,.mc .p-srv{animation-timing-function:steps(6)}
'''

# ---------- 3. Strict SaaS ----------
SAAS = '''
body.saas{font-family:"Geist","Inter",system-ui,sans-serif;color:#ededef;background:#09090b;font-size:14px}
.saas h1,.saas h2,.saas h3,.saas h4{font-weight:600;letter-spacing:-.01em}.saas .v{font-weight:600;letter-spacing:-.03em}
.saas .bg{position:fixed;inset:0;z-index:-1;background:radial-gradient(900px 500px at 50% -200px,rgba(120,119,198,.16),transparent),#09090b}
.saas .bg::after{content:"";position:absolute;inset:0;background-image:linear-gradient(rgba(255,255,255,.025) 1px,transparent 1px),linear-gradient(90deg,rgba(255,255,255,.025) 1px,transparent 1px);background-size:56px 56px;-webkit-mask:radial-gradient(ellipse at top,#000,transparent 70%);mask:radial-gradient(ellipse at top,#000,transparent 70%)}
.saas .app,.saas .phone{background:#0c0c0f;border:1px solid #1f1f23;border-radius:14px;box-shadow:0 0 0 1px #000,0 24px 60px -24px #000}
.saas .phone{border-radius:40px;padding:8px}.saas .screen{border-radius:32px;background:#09090b;border:1px solid #1f1f23}
.saas .rail{border-right:1px solid #1f1f23}
.saas .rail .logo{border-radius:10px;background:#ededef;color:#09090b}
.saas .rail a{border-radius:10px;color:#71717a;transition:color .15s,background .15s}
.saas .rail a:hover{color:#ededef;background:#18181b}
.saas .rail a.on{color:#ededef;background:#18181b}
.saas .rail a.on::before{content:"";position:absolute;left:-12px;top:14px;bottom:14px;width:2px;border-radius:2px;background:#ededef}
.saas .top{border-bottom:1px solid #1f1f23;margin-bottom:16px}
.saas .cmd,.saas .iconbtn,.saas .seg{background:#111113;border:1px solid #232327;border-radius:9px;color:#71717a;transition:border-color .15s}
.saas .cmd:hover{border-color:#3f3f46}
.saas .cmd kbd{border:1px solid #2e2e33;border-radius:5px;color:#a1a1aa;background:#18181b}
.saas .iconbtn .badge{background:#3b82f6}
.saas .kpi,.saas .srv,.saas .panel,.saas .p-srv,.saas .p-hero{background:#0f0f12;border:1px solid #1f1f23;border-radius:12px;transition:border-color .2s,transform .3s var(--ease)}
.saas .kpi::before,.saas .srv::before,.saas .panel::before{content:"";position:absolute;inset:0;border-radius:inherit;pointer-events:none;background:radial-gradient(300px circle at var(--mx,-200px) var(--my,-200px),rgba(255,255,255,.06),transparent 45%)}
.saas .panel{position:relative}
.saas .kpi .k,.saas .kpi .d,.saas .srv .nm p,.saas .meters span,.saas .up,.saas .off-note,.saas .p-srv small,.saas .p-top small,.saas .p-hero span,.saas .feed time{color:#71717a}
.saas .spark .line{stroke:#a1a1aa;stroke-width:1.5}.saas .spark .area{fill:#a1a1aa;opacity:.06}
.saas .kpi:nth-child(3) .line{stroke:#3b82f6}.saas .kpi:nth-child(3) .area{fill:#3b82f6;opacity:.1}
.saas .srv:hover{border-color:#3f3f46;transform:translateY(-2px)}
.saas .srv-art{display:none}
.saas .dot{width:8px;height:8px;background:#52525b;color:#52525b}.saas .running .dot{background:#22c55e;color:#22c55e}.saas .starting .dot{background:#eab308;color:#eab308}
.saas .pill{border-radius:6px;border:1px solid #27272a;color:#a1a1aa}.saas .running .pill{color:#4ade80;border-color:#14532d;background:#052e1680}.saas .starting .pill{color:#facc15;border-color:#713f12;background:#42200680}
.saas .bar{height:4px;background:#1f1f23;border-radius:4px}.saas .bar em{background:#ededef;border-radius:4px}
.saas .btn{border-radius:8px;font-weight:500}.saas .btn.primary{background:#ededef;color:#09090b}.saas .btn.primary:hover{background:#fff}
.saas .btn.ghost{border:1px solid #27272a;color:#a1a1aa}.saas .btn.ghost:hover{background:#18181b;color:#ededef}
.saas .seg button{border-radius:6px;color:#71717a}.saas .seg .on{background:#27272a;color:#ededef}
.saas .add{border:1px dashed #27272a;border-radius:12px;color:#71717a}.saas .add:hover{border-color:#52525b;color:#ededef}
.saas .feed li{position:relative;padding-left:2px}
.saas .feed .fi{border-radius:8px;border:1px solid #27272a;color:#a1a1aa}.saas .feed .err .fi{color:#f87171;border-color:#450a0a}.saas .feed .warn .fi{color:#facc15;border-color:#422006}.saas .feed .ok .fi{color:#4ade80;border-color:#052e16}
.saas .p-nav{border-top:1px solid #1f1f23;margin:10px 0 0}.saas .p-nav a{color:#71717a}.saas .p-nav a.on{color:#ededef}
.saas .fab{border-radius:14px;background:#ededef;color:#09090b}
.saas .p-hero .line{stroke:#3b82f6}.saas .p-hero .area{fill:#3b82f6;opacity:.1}
.saas .cap h1{color:#a1a1aa}
'''

VARIANTS = [
    ('glass', '1 · Glass / Aurora', GLASS, 'family=Inter:wght@400;500;600&family=Space+Grotesk:wght@500;600'),
    ('mc', '2 · Minecraft', MC, 'family=Press+Start+2P&family=Rubik:wght@400;500;600'),
    ('saas', '3 · Строгий SaaS', SAAS, 'family=Geist:wght@400;500;600&family=Inter:wght@400;500;600'),
]
for slug, title, css, fonts in VARIANTS:
    (ROOT / f'{slug}.html').write_text(page(slug, title, css, fonts), encoding='utf-8')
    print('ok', slug)
