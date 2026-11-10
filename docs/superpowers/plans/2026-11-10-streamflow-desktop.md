# StreamFlow Implementation Plan (етапи 1–2: macOS M1 + Windows 10+/Linux)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Десктопний застосунок на Tauri 2, який аналізує посилання через yt-dlp, дає вибір якості відео й аудіо (за замовчуванням найкраща) і завантажує один файл у Downloads.

**Architecture:** Чистий Rust-crate `streamflow-core` (probe, args, progress, tools, download, queue, settings) без залежності від Tauri; тонка оболонка `src-tauri` з командами й подіями; UI на TypeScript + Svelte. yt-dlp і ffmpeg запускаються як окремі процеси.

**Tech Stack:** Rust (tokio, reqwest+rustls, serde, sha2, flate2, dirs, thiserror), Tauri 2 (+ плагіни opener, notification, dialog), Svelte 5 + Vite + TypeScript, vitest, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-10-streamflow-desktop-design.md`

**Поза цим планом:** етап 3 (Legacy для Windows 7/8) робиться окремим планом після перевірки здійсненності на вашому старому ноутбуці. Ядро `streamflow-core` спеціально незалежне від Tauri, щоб Legacy його перевикористав.

## Global Constraints

- Tauri 2; логіка в crate `streamflow-core` без залежності від Tauri; UI: TypeScript + Svelte (Vite).
- Платформи: macOS Apple Silicon (перший), Windows 10+ 32/64-біт, Linux x64.
- За замовчуванням `-f bv*+ba/b` (найкраще відео + найкраще аудіо, один файл); «лише аудіо» за замовчуванням mp3; контейнер за замовчуванням mp4 з перемикачем mkv; завжди `--no-playlist`.
- Папка збереження: системна Downloads; шаблон назви `%(title)s.%(ext)s`.
- Пошук інструментів: системна копія з PATH, потім власна у папці даних застосунку; завантажені файли перевіряються за SHA-256.
- Пауза = зупинка процесу + відновлення з `--continue` (`.part` лишається).
- Поле введення: «Вставте посилання (наприклад, https://...)», без згадки YouTube.
- Статуси: Очікує, Завантажується, Обробка, Пауза, Зупинено, Готово, Помилка. Мови UI: українська й англійська, автовибір за мовою ОС.
- Паралельних завантажень за замовчуванням 2 (налаштовується).
- Кожен коміт закінчується рядком `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.

## Review Focus

1. Вставлено текст із пробілами/переносами або не-посилання (`"привіт"`): обрізати пробіли, показати «це не схоже на посилання», yt-dlp не запускати. → Task 2.
2. Назва файлу з пробілами, кирилицею, емодзі: шлях із `sf-file:` не втрачає символів, Open працює. → Task 4.
3. Джерело без окремих аудіодоріжок (лише змішаний формат) або лише аудіо (SoundCloud): списки не порожні, завантаження працює через `/b`. → Task 2, 3.
4. Невідомий розмір (стрім, HLS): у списку «—», а не `0 B`; прогрес без total показується невизначеною смугою. → Task 2, 4, 12.
5. Файл уже існує: завдання завершується «Готово» зі шляхом існуючого файлу, без помилки й без перезапису. → Task 4.

---

## Структура файлів

```
streamflow/                     (корінь проєкту, git init у Task 1)
  Cargo.toml                    workspace: core, src-tauri
  core/src/{lib,model,error,paths,settings,probe,args,progress,errors_classify,runner,queue}.rs
  core/src/tools/{mod,manifest,install}.rs
  core/tests/fixtures/*.json, fake_ytdlp.sh
  core/tests/integration_real.rs
  src-tauri/{Cargo.toml,tauri.conf.json,capabilities/default.json,src/{main,lib,commands,state}.rs}
  ui/{package.json,vite.config.ts,index.html}
  ui/src/{main.ts,App.svelte,app.css}
  ui/src/lib/{types,api,i18n,format}.ts
  ui/src/components/{UrlBar,MediaCard,QualityPicker,QueuePanel,JobCard,StatusBar,SettingsDialog,ToolsBanner}.svelte
  .github/workflows/{ci,release}.yml
```

---

### Task 1: Каркас workspace і модель даних

**Files:** Create `Cargo.toml`, `core/Cargo.toml`, `core/src/{lib,model,error}.rs`, `.gitignore`.

**Interfaces:**
- Produces (`model.rs`, усе `Serialize + Deserialize + Clone + Debug`, `#[serde(rename_all="camelCase")]`):
  `Format{format_id,ext,vcodec:Option<String>,acodec:Option<String>,height:Option<u32>,tbr:Option<f64>,abr:Option<f64>,filesize:Option<u64>,filesize_approx:Option<u64>}`,
  `VideoInfo{id,title,uploader:Option<String>,duration_secs:Option<u64>,thumbnail:Option<String>,webpage_url,is_playlist:bool,formats:Vec<Format>}`,
  `VideoChoice{format_id,height:u32,codec_label:String,size_bytes:Option<u64>}`,
  `AudioChoice{format_id,abr:u32,codec_label:String,size_bytes:Option<u64>}`,
  `enum VideoSelect{Best,Format(String),None}`, `enum AudioSelect{Best,Format(String)}`, `enum AudioOnlyFormat{Mp3,M4a,Opus}`, `enum Container{Mp4,Mkv}`,
  `DownloadRequest{url,title,thumbnail:Option<String>,video:VideoSelect,audio:AudioSelect,audio_only_format:AudioOnlyFormat}`,
  `enum JobState{Queued,Downloading,Processing,Paused,Stopped,Done,Failed}`,
  `Progress{downloaded:u64,total:Option<u64>,speed_bps:Option<f64>,eta_secs:Option<u64>}`.
- `error.rs`: `enum ErrorKind{Network,Unsupported,LoginRequired,Drm,DiskFull,NeedsUpdate,ToolMissing,InvalidUrl,Other}` і `struct UserError{kind:ErrorKind,detail:String}`.

- [ ] **Step 1: Перевірити інструменти.** Run: `rustc --version && cargo --version && node --version && xcode-select -p`. Expected: Rust ≥ 1.80, Node ≥ 20, шлях до Xcode CLT. Якщо чогось немає, зупинитись і сказати користувачу, що саме встановити (rustup, nvm); нічого не ставити мовчки.
- [ ] **Step 2: Failing test** у `core/src/model.rs` (`#[cfg(test)]`): `request_roundtrips_as_camel_case` серіалізує `DownloadRequest{video:VideoSelect::Best,..}` і перевіряє, що JSON містить `"audioOnlyFormat":"mp3"` після `Default` для `AudioOnlyFormat` і що десеріалізація повертає рівний об'єкт.
- [ ] **Step 3: Run** `cargo test -p streamflow-core` → FAIL (типів немає).
- [ ] **Step 4: Реалізувати** типи вище; `AudioOnlyFormat` реалізує `Default = Mp3`; `Container` `Default = Mp4`.
- [ ] **Step 5: Run** → PASS. `git init`, `.gitignore` (`target/`, `node_modules/`, `dist/`), commit `feat: scaffold workspace and core model`.

### Task 2: Probe — розбір JSON yt-dlp і списки якостей

**Files:** Create `core/src/probe.rs`, `core/tests/fixtures/{probe_full.json,probe_combined_only.json,probe_audio_only.json,probe_unknown_size.json}`.

**Interfaces:**
- Consumes: `model::*`.
- Produces: `pub fn normalize_url(input:&str) -> Result<String, UserError>` (trim, має починатися з `http://`/`https://`, інакше `InvalidUrl`);
  `pub fn parse_probe(json:&str) -> Result<VideoInfo, UserError>` (`_type=="playlist"` → `is_playlist=true`, `formats` береться з першого запису `entries`);
  `pub fn video_choices(info:&VideoInfo) -> Vec<VideoChoice>`;
  `pub fn audio_choices(info:&VideoInfo) -> Vec<AudioChoice>`;
  `pub fn estimate_total(info:&VideoInfo, v:&VideoSelect, a:&AudioSelect) -> Option<u64>`;
  `pub async fn fetch(ytdlp:&Path, url:&str, cookies:Option<&str>) -> Result<VideoInfo, UserError>` (запускає `yt-dlp -J --no-playlist [--cookies-from-browser <b>] <url>`).

Фікстура `probe_full.json` (скорочений реальний `yt-dlp -J`, `formats`): `313` vp9 2160p filesize 1_500_000_000; `137` avc1 1080p 300_000_000; `136` avc1 720p лише `filesize_approx` 150_000_000; `134` avc1 360p; `251` opus abr 160 filesize 5_000_000; `140` m4a abr 128 filesize 4_000_000; `18` змішаний avc1+mp4a 360p. Інші фікстури: лише змішаний `18`; лише аудіо (`251`,`140`, без відео); відео без жодного розміру.

- [ ] **Step 1: Failing tests** (`probe.rs`):
  - `normalize_url_trims_and_rejects_text`: `"  https://a.b/c\n"` → `"https://a.b/c"`; `"привіт"` → `InvalidUrl`.
  - `video_choices_one_per_height_sorted_desc`: висоти `[2160,1080,720,360]`; для 1080 `format_id=="137"`, `size_bytes==Some(300_000_000)`; для 720 `Some(150_000_000)` (із approx).
  - `audio_choices_sorted_by_abr`: `["251","140"]`, першому `codec_label=="Opus 160k"`.
  - `estimate_total_best_adds_best_video_and_audio`: `Some(1_505_000_000)`.
  - `combined_only_still_offers_video_choice`: з `probe_combined_only.json` `video_choices` має один елемент `360p`, `audio_choices` порожній (UI тоді ховає список аудіо).
  - `audio_only_source_has_no_video_choices`.
  - `unknown_size_is_none_not_zero`: `size_bytes==None` для всіх з `probe_unknown_size.json`, `estimate_total==None`.
  - `playlist_is_flagged`.
- [ ] **Step 2: Run** `cargo test -p streamflow-core probe` → FAIL.
- [ ] **Step 3: Implement** функції. Вибір формату на одну висоту: найбільший `tbr`. Лейбл кодека: `vp9→"VP9"`, `avc1*→"H.264"`, `av01*→"AV1"`, `opus→"Opus"`, `mp4a*→"AAC"`; аудіо `"<Кодек> <abr>k"`. `fetch` реалізувати через `tokio::process::Command`, помилку класифікувати функцією з Task 5 (поки що `ErrorKind::Other`, Task 5 підставляє класифікацію).
- [ ] **Step 4: Run** → PASS. Commit `feat(core): probe parsing and quality choices`.

### Task 3: Аргументи yt-dlp

**Files:** Create `core/src/args.rs`.

**Interfaces:**
- Consumes: `model::*`.
- Produces: `pub fn build_download_args(req:&DownloadRequest, out_dir:&Path, container:Container, ffmpeg:Option<&Path>, cookies:Option<&str>) -> Vec<String>`. Постійні аргументи: `--no-playlist --newline --progress --no-warnings`, `--progress-template "download:sf-progress:%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s"`, `--progress-template "postprocess:sf-post:%(progress.postprocessor)s"`, `--print "after_move:sf-file:%(filepath)s"`, `-o "<out_dir>/%(title)s.%(ext)s"`, на кінці `req.url`.

- [ ] **Step 1: Failing tests:**
  - `best_default`: містить послідовно `["-f","bv*+ba/b"]` і `["--merge-output-format","mp4"]`.
  - `mkv_container`: `["--merge-output-format","mkv"]`.
  - `explicit_formats`: `Format("313")`+`Format("251")` → `["-f","313+251"]`; `Format("313")`+`Best` → `"313+ba/b"`.
  - `audio_only_mp3`: `video=None` → містить `-x`, `["--audio-format","mp3"]`, `["--audio-quality","0"]`, `["-f","ba/b"]`; для `Format("251")` — `["-f","251"]`; немає `--merge-output-format`.
  - `ffmpeg_and_cookies`: `["--ffmpeg-location","/x/ffmpeg"]`, `["--cookies-from-browser","safari"]`.
  - `always_no_playlist`.
- [ ] **Step 2: Run** `cargo test -p streamflow-core args` → FAIL.
- [ ] **Step 3: Implement** `build_download_args`.
- [ ] **Step 4: Run** → PASS. Commit `feat(core): yt-dlp argument builder`.

### Task 4: Розбір рядків виводу yt-dlp

**Files:** Create `core/src/progress.rs`.

**Interfaces:**
- Produces: `pub enum LineEvent{Progress(Progress), PostProcess(String), File(PathBuf), Other}` і `pub fn parse_line(line:&str) -> LineEvent`.

- [ ] **Step 1: Failing tests:**
  - `parses_progress`: `"sf-progress:1048576|4194304|NA|524288.0|6"` → `downloaded 1048576`, `total Some(4194304)`, `speed_bps Some(524288.0)`, `eta Some(6)`.
  - `falls_back_to_estimate`: `"sf-progress:100|NA|2000|NA|NA"` → `total Some(2000)`, `speed None`, `eta None`.
  - `unknown_total_is_none`: `"sf-progress:100|NA|NA|NA|NA"` → `total None`.
  - `file_keeps_spaces_cyrillic_emoji`: `"sf-file:/Users/t/Downloads/Привіт світ 🎬.mp4"` → `File` з тим самим шляхом.
  - `already_downloaded_becomes_file`: `"[download] /tmp/My Clip.mp4 has already been downloaded"` → `File("/tmp/My Clip.mp4")`.
  - `postprocess`: `"sf-post:Merger"` → `PostProcess("Merger")`; довільний рядок → `Other`.
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement** `parse_line` (`NA` → `None`; `total` = `total_bytes` або `total_bytes_estimate`).
- [ ] **Step 4: Run** → PASS. Commit `feat(core): parse yt-dlp output lines`.

### Task 5: Класифікація помилок

**Files:** Create `core/src/errors_classify.rs`; modify `core/src/probe.rs` (`fetch` викликає класифікатор).

**Interfaces:** Produces: `pub fn classify_stderr(stderr:&str) -> UserError`.

- [ ] **Step 1: Failing tests** (`ErrorKind` для рядка → очікуваний): `"ERROR: Unsupported URL: https://x.y"`→`Unsupported`; `"Sign in to confirm you're not a bot"` і `"This video is only available for registered users"`→`LoginRequired`; `"This video is DRM protected"`→`Drm`; `"[Errno 28] No space left on device"`→`DiskFull`; `"Unable to download webpage: <urlopen error [Errno 8] nodename nor servname provided>"` і `"Temporary failure in name resolution"`→`Network`; `"ERROR: [site] abc: Unable to extract initial data"`→`NeedsUpdate`; все інше→`Other` із початковим текстом у `detail`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** за підрядками (без регулярних виразів, регістр ігнорується). **Step 4: Run** → PASS. Commit `feat(core): classify yt-dlp errors`.

### Task 6: Шляхи та налаштування

**Files:** Create `core/src/{paths,settings}.rs`.

**Interfaces:**
- Produces: `pub fn downloads_dir() -> PathBuf` (`dirs::download_dir()`, запасний `~/Downloads`); `pub fn tools_dir(app:&str) -> PathBuf` (`dirs::data_dir()/<app>/bin`);
  `pub struct Settings{output_dir:Option<PathBuf>, language:Language(Auto|Uk|En), theme:Theme(Auto|Light|Dark), max_parallel:u8 (=2), cookies_from:CookieSource(None|Safari|Chrome|Firefox), container:Container}` з `Default`;
  `impl Settings { pub fn load(path:&Path)->Settings; pub fn save(&self,path:&Path)->io::Result<()>; pub fn effective_output_dir(&self)->PathBuf; pub fn cookies_arg(&self)->Option<&'static str> }`.

- [ ] **Step 1: Failing tests:** `defaults`: `max_parallel==2`, `container==Mp4`, `effective_output_dir()==downloads_dir()`; `roundtrip` через `tempfile`; `corrupt_file_gives_defaults` (файл із `"{not json"`); `max_parallel_clamped` (0→1, 20→6 при `load`); `cookies_arg_values` (`Safari→Some("safari")`).
- [ ] **Step 2–4:** FAIL → implement → PASS. Commit `feat(core): paths and settings`.

### Task 7: Інструменти — маніфест, пошук, автоустановка

**Files:** Create `core/src/tools/{mod,manifest,install}.rs`.

**Interfaces:**
- Produces: `enum ToolId{YtDlp,Ffmpeg}`; `struct Platform{os:Os,arch:Arch}` з `Platform::current()`; 
  `manifest::source(tool:ToolId, p:Platform) -> Option<ToolSource>`, `ToolSource{url:String, sha256:Sha256Source(Fixed(String)|SumsFile(String)), gunzip:bool, file_name:&'static str}`;
  `tools::locate(tool:ToolId, tools_dir:&Path) -> Option<PathBuf>` (PATH, потім `tools_dir`);
  `install::install(tool:ToolId, p:Platform, tools_dir:&Path, on_progress:impl Fn(u64,Option<u64>)) -> Result<PathBuf, UserError>` (завантаження в тимчасовий файл, перевірка SHA-256, `gunzip` при потребі, `chmod 755` на unix, атомарне перейменування; при невідповідності хешу помилка й жодного файлу в `tools_dir`);
  `tools::manual_hint(tool:ToolId, p:Platform) -> String`; `tools::status(tools_dir:&Path) -> ToolsStatus{ytdlp:Option<PathBuf>, ffmpeg:Option<PathBuf>}`.

- [ ] **Step 1: Зафіксувати джерела (дослідження, результат записати в `manifest.rs`).** Run: `curl -s https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest | grep browser_download_url` і те саме для `eugeneware/ffmpeg-static`. Expected: назви активів. Для yt-dlp очікуються `yt-dlp_macos`, `yt-dlp.exe`, `yt-dlp_x86.exe`, `yt-dlp_linux`, `yt-dlp_linux_aarch64` і `SHA2-256SUMS` (хеші брати з нього). Для ffmpeg взяти активи тегу `b6.1.1` для `darwin-arm64`, `win32-ia32`, `win32-x64`, `linux-x64` і обчислити SHA-256 (`shasum -a 256`), вписати як `Fixed`. Якщо `yt-dlp_x86.exe` у релізі немає, `source(YtDlp, Windows x86)` повертає `None` і застосунок показує `manual_hint`; зафіксувати це в коментарі в маніфесті й повідомити користувача.
- [ ] **Step 2: Failing tests:** `manifest_covers_primary_platforms` (macOS arm64, Windows x64, Linux x64 мають джерело для обох інструментів); `locate_prefers_path_then_managed` (підкладений фейковий файл у `tools_dir`); `install_rejects_bad_hash` (локальний mock HTTP-сервер `httpmock` віддає файл із неправильним хешем → `Err`, `tools_dir` порожня); `install_ok_sets_executable` (правильний хеш → файл існує, на unix `mode & 0o111 != 0`); `install_gunzips`; `manual_hint_per_os` (`brew install yt-dlp ffmpeg` / `winget install yt-dlp ffmpeg` / `apt install yt-dlp ffmpeg`).
- [ ] **Step 3: Run** `cargo test -p streamflow-core tools` → FAIL. **Step 4: Implement**. `reqwest` з `rustls-tls`, потокове читання для `on_progress`.
- [ ] **Step 5: Run** → PASS. Commit `feat(core): tool discovery and verified auto-install`.

### Task 8: Запуск завантаження (runner)

**Files:** Create `core/src/runner.rs`, `core/tests/fixtures/fake_ytdlp.sh`.

**Interfaces:**
- Consumes: `args::build_download_args`, `progress::parse_line`, `errors_classify::classify_stderr`.
- Produces: `pub enum RunEvent{Progress(Progress),Processing,File(PathBuf)}`;
  `#[async_trait] pub trait JobRunner: Send+Sync { async fn run(&self, req:&DownloadRequest, on_event:&(dyn Fn(RunEvent)+Send+Sync), cancel:CancellationToken) -> Result<PathBuf, RunError>; }` де `enum RunError{Cancelled, Failed(UserError)}`;
  `pub struct YtDlpRunner{ytdlp:PathBuf, ffmpeg:Option<PathBuf>, out_dir:PathBuf, container:Container, cookies:Option<String>}` реалізує `JobRunner`.

`fake_ytdlp.sh` друкує канонічні рядки (`sf-progress:...`, `sf-post:Merger`, `sf-file:/tmp/x y.mp4`) і завершується кодом 0; режим за змінною `FAKE_MODE`: `ok`, `fail_login` (stderr `Sign in to confirm`, код 1), `slow` (`sleep 30`).

- [ ] **Step 1: Failing tests** (`#[cfg(unix)]`, tokio): `run_streams_progress_then_file` (події йдуть у порядку Progress → Processing → File, результат шляху з пробілом); `run_failure_is_classified` (`fail_login` → `Failed(UserError{kind:LoginRequired})`); `cancel_kills_child` (`slow`, скасування через 200 мс → `Cancelled`, процес завершено протягом 2 с); `exit_zero_without_file_line_is_error_unless_already_downloaded`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** через `tokio::process::Command` із `kill_on_drop(true)`, читання stdout рядками, stderr збирається. **Step 4: Run** → PASS. Commit `feat(core): yt-dlp runner with cancel`.

### Task 9: Черга

**Files:** Create `core/src/queue.rs`.

**Interfaces:**
- Consumes: `JobRunner`, `RunEvent`.
- Produces: `pub struct JobSnapshot{id:String, request:DownloadRequest, state:JobState, progress:Option<Progress>, file_path:Option<String>, error:Option<UserError>}`;
  `pub struct Queue` з `Queue::new(runner:Arc<dyn JobRunner>, max_parallel:usize, on_change:Arc<dyn Fn(JobSnapshot)+Send+Sync>)`, методами `add(req)->String`, `pause(id)`, `resume(id)`, `stop(id)`, `remove(id)`, `list()->Vec<JobSnapshot>`, `set_max_parallel(n)`.
  Семантика: `pause`/`stop` скасовують запуск, стани `Paused`/`Stopped`; `resume` для `Paused` і `Stopped` повертає в `Queued` і запускає знову (докачування робить yt-dlp). Кожна зміна викликає `on_change`.

- [ ] **Step 1: Failing tests** (з `FakeRunner`, керованим каналами): `respects_max_parallel` (3 завдання при `max=2` → два `Downloading`, одне `Queued`, після завершення першого третє стартує); `pause_then_resume_reruns`; `stop_is_terminal_until_resume`; `failure_sets_failed_with_error`; `done_has_file_path`; `remove_cancels_running`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** (tokio-задачі, `Mutex<HashMap>`, `CancellationToken` на завдання). **Step 4: Run** → PASS. Commit `feat(core): download queue`.

### Task 10: Оболонка Tauri

**Files:** Create `src-tauri/{Cargo.toml,tauri.conf.json,capabilities/default.json,src/{main,lib,commands,state}.rs}`.

**Interfaces:**
- Consumes: усе з `streamflow-core`.
- Produces (Tauri-команди; помилки як `UserError` у JSON): `analyze(url:String)->VideoProbe{info:VideoInfo,video:Vec<VideoChoice>,audio:Vec<AudioChoice>,estimateBest:Option<u64>}`; `start(req:DownloadRequest)->String`; `pause(id)`, `resume(id)`, `stop(id)`, `remove(id)`; `list_jobs()->Vec<JobSnapshot>`; `open_job(id)` (reveal у файловому менеджері через плагін opener); `get_settings()->Settings`, `set_settings(s:Settings)`; `tools_status()->ToolsStatus`, `tools_install(tool)`, `tools_update()`; події `job-changed(JobSnapshot)`, `tools-progress{tool,downloaded,total}`.
- `tauri.conf.json`: `productName "StreamFlow"`, `identifier "app.streamflow.desktop"`, вікно 900×720 без стандартної рамки не потрібне (звичайне вікно), `frontendDist "../ui/dist"`, `devUrl "http://localhost:5173"`, цілі збірки `dmg`, `nsis`, `appimage`, `deb`.

- [ ] **Step 1: Failing test** (`state.rs`): `app_state_builds_runner_from_settings_and_tools` на тимчасовій папці з фейковими `yt-dlp`/`ffmpeg`: стан повертає `ToolMissing`, коли yt-dlp немає, і створює `YtDlpRunner` із `out_dir == settings.effective_output_dir()`, коли є.
- [ ] **Step 2: Run** `cargo test -p streamflow-app` → FAIL. **Step 3: Implement** `AppState` і команди як тонкі обгортки; `analyze` викликає `normalize_url` → `fetch` → `video_choices/audio_choices/estimate_total`. Сповіщення при `Done` через плагін notification.
- [ ] **Step 4: Run** → PASS; `cargo check -p streamflow-app` без помилок. Commit `feat(app): tauri shell with commands and events`.

### Task 11: UI — каркас, типи, API, i18n, форматування

**Files:** Create `ui/{package.json,vite.config.ts,index.html}`, `ui/src/{main.ts,app.css}`, `ui/src/lib/{types,api,i18n,format}.ts` і їхні `*.test.ts`.

**Interfaces:**
- Produces: `types.ts` дзеркало Rust-типів (camelCase). `api.ts`: `analyze(url)`, `start(req)`, `pause/resume/stop/remove/openJob(id)`, `listJobs()`, `getSettings()/setSettings(s)`, `toolsStatus()/toolsInstall(tool)`, `onJobChanged(cb)`, `onToolsProgress(cb)` поверх `@tauri-apps/api` `invoke`/`listen`.
  `i18n.ts`: `t(key, params?)`, `setLanguage('auto'|'uk'|'en')` (auto за `navigator.language`, запасна `en`). `format.ts`: `formatBytes(n?:number|null)` (`null`→`"—"`), `formatSpeed`, `formatEta`, `formatPercent(p:Progress)` (`total==null`→`null`).

- [ ] **Step 1: Failing tests (vitest):** `formatBytes(1_600_000_000)==="1.6 GB"` (uk: `"1,6 ГБ"`), `formatBytes(null)==="—"`, `formatBytes(0)==="0 B"`; `formatPercent({downloaded:50,total:200})===25`, з `total:null` → `null`; `t('pasteLink')` в uk дорівнює `"Вставте посилання (наприклад, https://...)"`; невідома мова → en; відсутній ключ повертає сам ключ.
- [ ] **Step 2: Run** `cd ui && npm test` → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS. Commit `feat(ui): scaffold, api, i18n, formatting`.

### Task 12: UI — введення, картка джерела, вибір якості

**Files:** Create `ui/src/components/{UrlBar,MediaCard,QualityPicker}.svelte`, `ui/src/App.svelte` (стан `idle→analyzing→ready→error`), тести поруч.

**Interfaces:** `UrlBar` генерує `submit(url)` (Enter або іконка пошуку); `MediaCard` показує мініатюру, заголовок, тривалість, `QualityPicker` і кнопку «Завантажити» (`download` подія з `DownloadRequest`). `QualityPicker` props: `video:VideoChoice[]`, `audio:AudioChoice[]`, `estimateBest`.

- [ ] **Step 1: Failing tests** (`@testing-library/svelte`): `urlbar_submits_on_enter`; `quality_defaults_to_best` (обидва списки мають вибрану першу опцію «Найкраща …» і приблизний розмір із `estimateBest`); `audio_only_shows_format_select` (вибір «Без відео» показує mp3/m4a/opus із mp3 за замовчуванням); `no_audio_choices_hides_audio_list` (аудіо порожнє → список аудіо відсутній); `unknown_size_shows_dash`; `download_emits_request_with_selects` (`Best/Best` → `{video:'best',audio:'best'}`; вибрано 1080p → `{video:{format:'137'}}`); `invalid_url_shows_message_without_calling_api` (мок `api.analyze` не викликано для `"привіт"`).
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** → PASS. Commit `feat(ui): url input, media card and quality pickers`.

### Task 13: UI — черга, статистика, налаштування, банер інструментів

**Files:** Create `ui/src/components/{QueuePanel,JobCard,StatusBar,SettingsDialog,ToolsBanner}.svelte`, стилі «темне скло» в `app.css` (за скріншотом: напівпрозорі картки, градієнтна смуга прогресу, світла/темна тема за ОС і налаштуванням), тести поруч.

**Interfaces:** `JobCard` кнопки за станом: `Downloading`/`Processing` → Pause, Stop, Open недоступний; `Paused`/`Stopped` → Resume, Open недоступний; `Done` → Open, Remove; `Failed` → Retry (`resume`), текст помилки з `t('error.<kind>')`; `Queued` → Stop. `StatusBar`: «Активні завантаження: N · Завершено: M · Всього: K» і Settings. `ToolsBanner`: при `tools_status` без yt-dlp/ffmpeg показує «Встановити автоматично» (прогрес із `tools-progress`) та `manual_hint`, якщо автоустановка не вдалася.

- [ ] **Step 1: Failing tests:** `jobcard_buttons_per_state` (таблиця вище); `status_bar_counts` (3 Downloading/Queued, 1 Done → `3`, `1`, `4`); `indeterminate_bar_when_total_null`; `pause_click_calls_api`; `failed_shows_localized_error` (LoginRequired → підказка про cookies із браузера); `settings_saves_output_dir_and_container`; `tools_banner_shows_install_and_manual_hint_on_failure`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** `npm test` → PASS. Commit `feat(ui): queue, status bar, settings, tools banner`.

### Task 14: Інтеграційна перевірка на справжньому yt-dlp і збірка для M1

**Files:** Create `core/tests/integration_real.rs`; modify `README.md` (створити коротко: запуск, збірка).

- [ ] **Step 1: Failing test** `#[ignore]` `real_download_has_video_and_audio`: `install` yt-dlp і ffmpeg у тимчасову папку, `fetch` для `https://www.youtube.com/watch?v=BaW_jenozKc`, завантажити з `Best/Best` у тимчасову папку, переконатися, що файл існує, і що `ffmpeg -i <file>` у stderr містить `Video:` і `Audio:`. Також `real_audio_only_mp3` (розширення `.mp3`).
- [ ] **Step 2: Run** `cargo test -p streamflow-core --test integration_real -- --ignored --nocapture` → спершу FAIL або PASS залежно від мережі; виправити розбіжності (наприклад, чи `--print after_move:` не глушить прогрес без `--progress`) в `args.rs` і оновити його тести з Task 3.
- [ ] **Step 3: Збірка:** `cd ui && npm ci && npm run build`, потім `cargo tauri build --target aarch64-apple-darwin`. Expected: `.dmg` у `src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/`.
- [ ] **Step 4: Ручна перевірка на M1** (відмітити кожен пункт): перший запуск без yt-dlp/ffmpeg (банер, автоустановка); вставити посилання, картка з двома списками «Найкраща»; «Завантажити» без змін → один файл з відео та аудіо у Downloads, Open показує його у Finder; пауза/відновлення; Stop; «Без відео» → mp3; посилання не з YouTube (X або TikTok); некоректний текст; відсутність мережі; тема світла/темна; мова uk/en.
- [ ] **Step 5: Commit** `test: real-download integration test and M1 build notes`.

### Task 15: CI для macOS / Windows (x64, x86) / Linux

**Files:** Create `.github/workflows/ci.yml`, `.github/workflows/release.yml`.

- [ ] **Step 1:** `ci.yml`: матриця `macos-14` (`aarch64-apple-darwin`), `windows-latest` (`x86_64-pc-windows-msvc` і `i686-pc-windows-msvc`), `ubuntu-22.04` (з `libwebkit2gtk-4.1-dev` та залежностями Tauri). Кроки: checkout, rust toolchain + target, node 20, `cargo test -p streamflow-core`, `npm ci && npm test` у `ui`, `cargo tauri build --target <матриця>` (без `integration_real`).
- [ ] **Step 2:** `release.yml` за тегом `v*` викликає `tauri-apps/tauri-action` для тієї ж матриці й додає артефакти до чернетки релізу.
- [ ] **Step 3: Перевірка:** `actionlint .github/workflows/*.yml` (якщо `actionlint` не встановлено, прочитати файли й звірити відступи та назви кроків). Реальний запуск потребує GitHub-репозиторію: повідомити користувача, що Windows/Linux-збірки не перевірені, поки CI не відпрацює.
- [ ] **Step 4: Commit** `ci: build and test matrix for macOS, Windows, Linux`.

---

## Self-review

- **Покриття специфікації:** розділи 4–5 → Tasks 1–13; 5 (таблиця відповідності yoinks) → Tasks 2–4, 11–13; 6 (інструменти) → Task 7 і банер Task 13; 7 (помилки) → Task 5 і `JobCard` Task 13; 8 (платформи) → Tasks 14–15; 10 (тестування) → у кожній задачі, Task 14, Task 15; 9 (Legacy) свідомо винесено окремим планом. Відкриті питання spec: контейнер → `Settings.container` (Task 6), мови → Task 11, назва/іконка → лишається робочою назвою (замінюється у `tauri.conf.json`), 32-біт джерела → Task 7 Step 1.
- **Відомі непевності, які планом не закриваються:** чи є `yt-dlp_x86.exe` у поточному релізі (Task 7 Step 1); чи не придушує `--print` прогрес без `--progress` (Task 14 Step 2); Windows/Linux збірки не перевіряються з Mac до першого запуску CI.
