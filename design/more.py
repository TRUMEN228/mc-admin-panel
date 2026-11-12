# Дополнительные варианты стилей (между Glass и SaaS).  python design/more.py
import importlib.util, pathlib, sys

ROOT = pathlib.Path(__file__).parent
spec = importlib.util.spec_from_file_location('build', ROOT / 'build.py')
b = importlib.util.module_from_spec(spec); sys.modules['build'] = b; spec.loader.exec_module(b)

FONTS = 'family=Inter:wght@400;500;600&family=Space+Grotesk:wght@500;600'

# 4. Гибрид: структура SaaS + стеклянные акценты (изумруд/бирюза)
HYBRID = '''
body.hybrid{font-family:"Inter",system-ui,sans-serif;color:#ececf1;background:#0a0b0f}
.hybrid h1,.hybrid h2,.hybrid h3,.hybrid h4,.hybrid .v{font-family:"Space Grotesk",sans-serif;font-weight:600;letter-spacing:-.01em}
.hybrid .bg{position:fixed;inset:0;z-index:-1;background:#0a0b0f;overflow:hidden}
.hybrid .bg::before{content:"";position:absolute;width:1100px;height:600px;left:50%;top:-380px;transform:translateX(-50%);background:radial-gradient(closest-side,#10b98155,#06b6d433 50%,transparent);filter:blur(40px);animation:breathe 9s ease-in-out infinite alternate}
@keyframes breathe{to{opacity:.55;transform:translateX(-50%) scale(1.15)}}
.hybrid .app,.hybrid .phone{background:rgba(16,17,23,.72);backdrop-filter:blur(20px);border:1px solid #23252e;border-radius:16px;box-shadow:0 30px 70px -30px #000}
.hybrid .phone{border-radius:42px;padding:9px}.hybrid .screen{border-radius:33px;background:#0c0d12;border:1px solid #23252e}
.hybrid .rail{border-right:1px solid #1d1f27}
.hybrid .rail .logo{border-radius:12px;background:linear-gradient(135deg,#10b981,#06b6d4);color:#04110c;box-shadow:0 6px 20px -6px #10b981}
.hybrid .rail a{border-radius:12px;color:#6b7080}.hybrid .rail a:hover{color:#ececf1;background:#171922}
.hybrid .rail a.on{color:#ececf1;background:#171922;box-shadow:inset 0 0 0 1px #2b2e3a}.hybrid .rail a.on .ic{color:#34d399}
.hybrid .top{border-bottom:1px solid #1d1f27;margin-bottom:16px}
.hybrid .cmd,.hybrid .iconbtn,.hybrid .seg{background:#12131a;border:1px solid #23252e;border-radius:10px;color:#6b7080}
.hybrid .cmd kbd{border:1px solid #2b2e3a;border-radius:5px;color:#a3a7b5}
.hybrid .iconbtn .badge{background:#34d399;box-shadow:0 0 8px #34d399}
.hybrid .kpi,.hybrid .srv,.hybrid .panel,.hybrid .p-srv,.hybrid .p-hero{background:radial-gradient(320px circle at var(--mx,-200px) var(--my,-200px),rgba(52,211,153,.08),transparent 45%),#111219;border:1px solid #20222b;border-radius:14px}
.hybrid .kpi .k,.hybrid .kpi .d,.hybrid .srv .nm p,.hybrid .meters span,.hybrid .up,.hybrid .off-note,.hybrid .p-srv small,.hybrid .p-top small,.hybrid .p-hero span,.hybrid .feed time{color:#6b7080}
.hybrid .spark .line{stroke:#34d399;stroke-width:1.8}.hybrid .spark .area{fill:#34d399;opacity:.08}
.hybrid .kpi:nth-child(2) .line{stroke:#22d3ee}.hybrid .kpi:nth-child(2) .area{fill:#22d3ee}
.hybrid .srv:hover{border-color:#2f6b57;box-shadow:0 16px 40px -20px #10b98199}
.hybrid .srv-art{height:2px;opacity:1}.hybrid .running .srv-art{background:linear-gradient(90deg,transparent,#34d399,transparent);animation:scan 3s linear infinite}.hybrid .starting .srv-art{background:linear-gradient(90deg,transparent,#fbbf24,transparent);animation:scan 1.5s linear infinite}
@keyframes scan{from{transform:translateX(-100%)}to{transform:translateX(100%)}}
.hybrid .dot{width:9px;height:9px;background:#4b4f5c;color:#4b4f5c}.hybrid .running .dot{background:#34d399;color:#34d399;box-shadow:0 0 10px #34d399}.hybrid .starting .dot{background:#fbbf24;color:#fbbf24}
.hybrid .pill{border-radius:7px;border:1px solid #2b2e3a;color:#8b90a0}.hybrid .running .pill{color:#6ee7b7;border-color:#134e3a;background:#06281d}.hybrid .starting .pill{color:#fcd34d;border-color:#5b4310;background:#2a1f06}
.hybrid .bar{height:4px;background:#1d1f27;border-radius:4px}.hybrid .bar em{border-radius:4px;background:linear-gradient(90deg,#10b981,#22d3ee)}
.hybrid .btn{border-radius:9px;font-weight:500}.hybrid .btn.primary{background:linear-gradient(135deg,#10b981,#06b6d4);color:#03130d;font-weight:600;box-shadow:0 6px 18px -8px #10b981}.hybrid .btn.primary:hover{filter:brightness(1.1)}
.hybrid .btn.ghost{border:1px solid #2b2e3a;color:#a3a7b5}.hybrid .btn.ghost:hover{background:#1a1c25;color:#fff}
.hybrid .seg button{border-radius:7px;color:#6b7080}.hybrid .seg .on{background:#20222b;color:#ececf1}
.hybrid .add{border:1px dashed #2b2e3a;border-radius:14px;color:#6b7080}.hybrid .add:hover{border-color:#34d399;color:#6ee7b7}
.hybrid .feed .fi{border-radius:9px;background:#171922;color:#a3a7b5}.hybrid .feed .ok .fi{color:#34d399}.hybrid .feed .warn .fi{color:#fbbf24}.hybrid .feed .err .fi{color:#f87171}.hybrid .feed .info .fi{color:#22d3ee}
.hybrid .p-nav{border-top:1px solid #1d1f27;margin:10px 0 0}.hybrid .p-nav a{color:#6b7080}.hybrid .p-nav a.on{color:#34d399}
.hybrid .fab{border-radius:16px;background:linear-gradient(135deg,#10b981,#06b6d4);color:#03130d;box-shadow:0 10px 26px -8px #10b981}
.hybrid .p-hero .line{stroke:#22d3ee}.hybrid .p-hero .area{fill:#22d3ee}.hybrid .cap h1{color:#6ee7b7}
'''

# 5. Светлая тема с матовым стеклом
LIGHT = '''
body.light{font-family:"Inter",system-ui,sans-serif;color:#0f172a;background:#eef1f7}
.light h1,.light h2,.light h3,.light h4,.light .v{font-family:"Space Grotesk",sans-serif;font-weight:600}
.light .bg{position:fixed;inset:0;z-index:-1;overflow:hidden;background:#eef1f7}
.light .bg::before,.light .bg::after{content:"";position:absolute;width:800px;height:800px;border-radius:50%;filter:blur(110px);opacity:.6;animation:lfloat 24s ease-in-out infinite alternate}
.light .bg::before{background:#a5b4fc;left:-200px;top:-300px}.light .bg::after{background:#99f6e4;right:-250px;bottom:-350px;animation-duration:30s}
@keyframes lfloat{to{transform:translate(140px,100px) scale(1.1)}}
.light .app,.light .phone{background:rgba(255,255,255,.62);backdrop-filter:blur(30px) saturate(160%);border:1px solid rgba(255,255,255,.9);border-radius:24px;box-shadow:0 30px 60px -30px rgba(30,41,59,.35),inset 0 1px 0 #fff}
.light .phone{border-radius:44px;padding:10px}.light .screen{border-radius:34px;background:rgba(248,250,252,.8)}
.light .rail{border-right:1px solid rgba(15,23,42,.06)}
.light .rail .logo{border-radius:14px;background:linear-gradient(135deg,#6366f1,#14b8a6);color:#fff;box-shadow:0 8px 20px -8px #6366f1}
.light .rail a{border-radius:14px;color:#64748b}.light .rail a:hover{background:rgba(255,255,255,.8);color:#0f172a}
.light .rail a.on{background:#fff;color:#4f46e5;box-shadow:0 4px 14px -6px rgba(30,41,59,.25)}
.light .cmd,.light .iconbtn,.light .seg{background:rgba(255,255,255,.75);border:1px solid rgba(15,23,42,.07);border-radius:12px;color:#64748b}
.light .cmd kbd{background:#f1f5f9;border-radius:6px;color:#475569}
.light .iconbtn .badge{background:#f43f5e}
.light .kpi,.light .srv,.light .panel,.light .p-srv,.light .p-hero{background:radial-gradient(360px circle at var(--mx,-200px) var(--my,-200px),rgba(99,102,241,.08),transparent 45%),rgba(255,255,255,.78);border:1px solid rgba(255,255,255,.95);border-radius:18px;box-shadow:0 6px 20px -12px rgba(30,41,59,.25)}
.light .kpi .k,.light .kpi .d,.light .srv .nm p,.light .meters span,.light .up,.light .off-note,.light .p-srv small,.light .p-top small,.light .p-hero span,.light .feed time{color:#64748b}
.light .spark .line{stroke:#6366f1}.light .spark .area{fill:#6366f1;opacity:.1}
.light .kpi:nth-child(2) .line{stroke:#14b8a6}.light .kpi:nth-child(2) .area{fill:#14b8a6}.light .kpi:nth-child(3) .line{stroke:#f59e0b}.light .kpi:nth-child(3) .area{fill:#f59e0b}.light .kpi:nth-child(4) .line{stroke:#ec4899}.light .kpi:nth-child(4) .area{fill:#ec4899}
.light .srv:hover{box-shadow:0 22px 40px -20px rgba(79,70,229,.45)}
.light .srv-art{-webkit-mask:linear-gradient(#000,transparent);mask:linear-gradient(#000,transparent);opacity:.5}.light .art-a{background:linear-gradient(90deg,#a5b4fc,transparent)}.light .art-b{background:linear-gradient(90deg,#99f6e4,transparent)}.light .art-c{background:linear-gradient(90deg,#fde68a,transparent)}
.light .dot{background:#cbd5e1;color:#cbd5e1}.light .running .dot{background:#10b981;color:#10b981}.light .starting .dot{background:#f59e0b;color:#f59e0b}
.light .pill{border-radius:99px;background:#f1f5f9;color:#64748b}.light .running .pill{background:#d1fae5;color:#047857}.light .starting .pill{background:#fef3c7;color:#b45309}
.light .bar{background:#e2e8f0;border-radius:99px}.light .bar em{border-radius:99px;background:linear-gradient(90deg,#6366f1,#14b8a6)}
.light .btn{border-radius:11px;font-weight:500}.light .btn.primary{background:#0f172a;color:#fff}.light .btn.primary:hover{background:#4f46e5}
.light .btn.ghost{background:#fff;color:#334155;box-shadow:0 1px 3px rgba(15,23,42,.12)}.light .btn.ghost:hover{background:#f8fafc}
.light .seg button{border-radius:9px;color:#64748b}.light .seg .on{background:#fff;color:#0f172a;box-shadow:0 1px 4px rgba(15,23,42,.12)}
.light .add{border:1.5px dashed #cbd5e1;border-radius:18px;color:#64748b}.light .add:hover{border-color:#6366f1;color:#4f46e5;background:rgba(255,255,255,.6)}
.light .feed .fi{border-radius:10px}.light .feed .ok .fi{background:#d1fae5;color:#059669}.light .feed .info .fi{background:#e0e7ff;color:#4f46e5}.light .feed .warn .fi{background:#fef3c7;color:#d97706}.light .feed .err .fi{background:#ffe4e6;color:#e11d48}
.light .p-nav{background:rgba(255,255,255,.85);border-radius:22px;box-shadow:0 6px 20px -10px rgba(30,41,59,.3)}.light .p-nav a{color:#64748b;border-radius:14px}.light .p-nav a.on{color:#4f46e5;background:#eef2ff}
.light .fab{border-radius:18px;background:#0f172a;color:#fff;box-shadow:0 12px 26px -10px #0f172a}
.light .p-hero .line{stroke:#14b8a6}.light .p-hero .area{fill:#14b8a6}.light .cap h1{color:#4f46e5}
'''

# 6. Тёмный с анимированными градиентными рамками
GLOW = '''
body.glow{font-family:"Inter",system-ui,sans-serif;color:#f1f1f4;background:#050507}
.glow h1,.glow h2,.glow h3,.glow h4,.glow .v{font-family:"Space Grotesk",sans-serif;font-weight:600}
.glow .bg{position:fixed;inset:0;z-index:-1;background:radial-gradient(1000px 500px at 80% -10%,#7c3aed33,transparent),radial-gradient(800px 500px at 0 100%,#f9731622,transparent),#050507}
@property --a{syntax:"<angle>";inherits:false;initial-value:0deg}
.glow .app,.glow .phone{background:#0b0b0f;border-radius:18px;position:relative;border:1px solid #1c1c22}
.glow .app::before,.glow .phone::before{content:"";position:absolute;inset:-1px;border-radius:inherit;padding:1px;background:conic-gradient(from var(--a),transparent 70%,#a855f7,#f97316,transparent 95%);-webkit-mask:linear-gradient(#000 0 0) content-box,linear-gradient(#000 0 0);-webkit-mask-composite:xor;mask-composite:exclude;animation:spin 6s linear infinite;pointer-events:none;z-index:2}
@keyframes spin{to{--a:360deg}}
.glow .phone{border-radius:44px;padding:9px}.glow .screen{border-radius:35px;background:#08080b}
.glow .rail{border-right:1px solid #18181d}
.glow .rail .logo{border-radius:12px;background:linear-gradient(135deg,#a855f7,#f97316);color:#fff}
.glow .rail a{border-radius:12px;color:#6e6e7a}.glow .rail a:hover{color:#fff;background:#141419}
.glow .rail a.on{color:#fff;background:linear-gradient(135deg,#a855f722,#f9731622);box-shadow:inset 0 0 0 1px #a855f755}
.glow .cmd,.glow .iconbtn,.glow .seg{background:#101014;border:1px solid #1f1f26;border-radius:10px;color:#6e6e7a}
.glow .cmd kbd{border:1px solid #2a2a33;border-radius:5px;color:#a1a1ad}
.glow .iconbtn .badge{background:#f97316;box-shadow:0 0 10px #f97316}
.glow .kpi,.glow .srv,.glow .panel,.glow .p-srv,.glow .p-hero{background:#0e0e12;border:1px solid #1d1d24;border-radius:14px;position:relative}
.glow .kpi::after,.glow .srv::after,.glow .panel::after{content:"";position:absolute;inset:-1px;border-radius:inherit;padding:1px;pointer-events:none;background:radial-gradient(220px circle at var(--mx,-300px) var(--my,-300px),#c084fc,#fb923c 40%,transparent 70%);-webkit-mask:linear-gradient(#000 0 0) content-box,linear-gradient(#000 0 0);-webkit-mask-composite:xor;mask-composite:exclude;opacity:0;transition:opacity .3s}
.glow .kpi:hover::after,.glow .srv:hover::after,.glow .panel:hover::after{opacity:1}
.glow .kpi .k,.glow .kpi .d,.glow .srv .nm p,.glow .meters span,.glow .up,.glow .off-note,.glow .p-srv small,.glow .p-top small,.glow .p-hero span,.glow .feed time{color:#6e6e7a}
.glow .spark .line{stroke:#c084fc}.glow .spark .area{fill:#c084fc;opacity:.08}
.glow .kpi:nth-child(even) .line{stroke:#fb923c}.glow .kpi:nth-child(even) .area{fill:#fb923c}
.glow .srv:hover{box-shadow:0 20px 50px -25px #a855f7}
.glow .srv-art{display:none}
.glow .dot{width:9px;height:9px;background:#3f3f46;color:#3f3f46}.glow .running .dot{background:#4ade80;color:#4ade80;box-shadow:0 0 10px #4ade80}.glow .starting .dot{background:#fb923c;color:#fb923c}
.glow .pill{border-radius:99px;border:1px solid #27272f;color:#8b8b96}.glow .running .pill{color:#86efac;border-color:#14532d}.glow .starting .pill{color:#fdba74;border-color:#7c2d12}
.glow .bar{height:4px;background:#1a1a20;border-radius:4px}.glow .bar em{border-radius:4px;background:linear-gradient(90deg,#a855f7,#f97316)}
.glow .btn{border-radius:9px;font-weight:500}.glow .btn.primary{background:linear-gradient(135deg,#a855f7,#f97316);color:#fff;font-weight:600;box-shadow:0 8px 24px -10px #a855f7}
.glow .btn.ghost{border:1px solid #27272f;color:#a1a1ad}.glow .btn.ghost:hover{background:#17171c;color:#fff}
.glow .seg button{border-radius:7px;color:#6e6e7a}.glow .seg .on{background:#1d1d24;color:#fff}
.glow .add{border:1px dashed #2a2a33;border-radius:14px;color:#6e6e7a}.glow .add:hover{border-color:#a855f7;color:#d8b4fe}
.glow .feed .fi{border-radius:9px;background:#141419;color:#a1a1ad}.glow .feed .ok .fi{color:#4ade80}.glow .feed .warn .fi{color:#fb923c}.glow .feed .err .fi{color:#f87171}.glow .feed .info .fi{color:#c084fc}
.glow .p-nav{border-top:1px solid #18181d;margin:10px 0 0}.glow .p-nav a{color:#6e6e7a}.glow .p-nav a.on{color:#fff}
.glow .fab{border-radius:16px;background:linear-gradient(135deg,#a855f7,#f97316);color:#fff;box-shadow:0 10px 30px -10px #a855f7}
.glow .p-hero .line{stroke:#fb923c}.glow .p-hero .area{fill:#fb923c}.glow .cap h1{background:linear-gradient(90deg,#c084fc,#fb923c);-webkit-background-clip:text;color:transparent}
'''

# 7. Спокойный, мягкие холодные тона
CALM = '''
body.calm{font-family:"Manrope","Inter",system-ui,sans-serif;color:#e5e9f0;background:#1f232b}
.calm h1,.calm h2,.calm h3,.calm h4,.calm .v{font-weight:700;letter-spacing:-.01em}
.calm .bg{position:fixed;inset:0;z-index:-1;background:linear-gradient(160deg,#252a34,#1b1e25)}
.calm .app,.calm .phone{background:#262b35;border-radius:22px;box-shadow:0 30px 60px -30px #0009,inset 0 1px 0 #ffffff0a}
.calm .phone{border-radius:44px;padding:10px}.calm .screen{border-radius:34px;background:#232731}
.calm .rail{background:#232731}
.calm .rail .logo{border-radius:14px;background:#88c0d0;color:#1f232b}
.calm .rail a{border-radius:14px;color:#7b8394}.calm .rail a:hover{color:#e5e9f0;background:#2c323d}
.calm .rail a.on{color:#88c0d0;background:#2e3440}
.calm .cmd,.calm .iconbtn,.calm .seg{background:#2e3440;border-radius:12px;color:#7b8394}
.calm .cmd kbd{background:#3b4252;border-radius:6px;color:#c0c6d2}
.calm .iconbtn .badge{background:#ebcb8b}
.calm .kpi,.calm .srv,.calm .panel,.calm .p-srv,.calm .p-hero{background:#2e3440;border-radius:18px;box-shadow:0 10px 24px -16px #000a}
.calm .kpi .k,.calm .kpi .d,.calm .srv .nm p,.calm .meters span,.calm .up,.calm .off-note,.calm .p-srv small,.calm .p-top small,.calm .p-hero span,.calm .feed time{color:#7b8394}
.calm .spark .line{stroke:#88c0d0;stroke-width:2.2}.calm .spark .area{fill:#88c0d0;opacity:.1}
.calm .kpi:nth-child(2) .line{stroke:#a3be8c}.calm .kpi:nth-child(2) .area{fill:#a3be8c}.calm .kpi:nth-child(3) .line{stroke:#ebcb8b}.calm .kpi:nth-child(3) .area{fill:#ebcb8b}.calm .kpi:nth-child(4) .line{stroke:#b48ead}.calm .kpi:nth-child(4) .area{fill:#b48ead}
.calm .srv:hover{transform:translateY(-5px);box-shadow:0 24px 40px -20px #000}
.calm .srv-art{display:none}
.calm .dot{background:#4c566a;color:#4c566a}.calm .running .dot{background:#a3be8c;color:#a3be8c}.calm .starting .dot{background:#ebcb8b;color:#ebcb8b}
.calm .pill{border-radius:99px;background:#3b4252;color:#9aa3b5}.calm .running .pill{background:#a3be8c22;color:#a3be8c}.calm .starting .pill{background:#ebcb8b22;color:#ebcb8b}
.calm .bar{height:6px;background:#3b4252;border-radius:99px}.calm .bar em{border-radius:99px;background:#88c0d0}
.calm .btn{border-radius:12px;font-weight:600}.calm .btn.primary{background:#88c0d0;color:#1f232b}.calm .btn.primary:hover{background:#8fbcbb}
.calm .btn.ghost{background:#3b4252;color:#d8dee9}.calm .btn.ghost:hover{background:#434c5e}
.calm .seg button{border-radius:9px;color:#7b8394}.calm .seg .on{background:#3b4252;color:#e5e9f0}
.calm .add{border:2px dashed #3b4252;border-radius:18px;color:#7b8394}.calm .add:hover{border-color:#88c0d0;color:#88c0d0}
.calm .feed .fi{border-radius:11px;background:#3b4252}.calm .feed .ok .fi{color:#a3be8c}.calm .feed .info .fi{color:#88c0d0}.calm .feed .warn .fi{color:#ebcb8b}.calm .feed .err .fi{color:#bf616a}
.calm .p-nav{background:#2e3440;border-radius:22px}.calm .p-nav a{color:#7b8394;border-radius:14px}.calm .p-nav a.on{color:#88c0d0;background:#3b4252}
.calm .fab{border-radius:18px;background:#88c0d0;color:#1f232b}
.calm .p-hero .line{stroke:#a3be8c}.calm .p-hero .area{fill:#a3be8c}.calm .cap h1{color:#88c0d0}
'''

MORE = [
    ('hybrid', '4 · Гибрид (SaaS + стекло)', HYBRID, FONTS),
    ('light', '5 · Светлый frosted', LIGHT, FONTS),
    ('glow', '6 · Градиентные рамки', GLOW, FONTS),
    ('calm', '7 · Спокойный', CALM, 'family=Manrope:wght@400;500;600;700'),
]
for slug, title, css, fonts in MORE:
    (ROOT / f'{slug}.html').write_text(b.page(slug, title, css, fonts), encoding='utf-8')
    print('ok', slug)
