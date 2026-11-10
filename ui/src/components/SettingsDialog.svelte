<script lang="ts">
  import { untrack } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import * as api from '../lib/api';
  import { tr } from '../lib/i18n';
  import { toUserError } from '../lib/errors';
  import type { Settings, ToolsStatus } from '../lib/types';

  interface Props {
    settings: Settings;
    tools: ToolsStatus;
    onsave: (settings: Settings) => void;
    onclose: () => void;
    ontools: (status: ToolsStatus) => void;
  }
  let { settings, tools, onsave, onclose, ontools }: Props = $props();

  const uid = $props.id();
  let draft = $state<Settings>(untrack(() => ({ ...settings })));
  let toolsNow = $state<ToolsStatus>(untrack(() => ({ ...tools })));
  let updating = $state(false);
  let updateError = $state('');

  async function chooseFolder() {
    const dir = await open({ directory: true, multiple: false, defaultPath: draft.outputDir ?? undefined });
    if (typeof dir === 'string') draft.outputDir = dir;
  }

  async function updateYtdlp() {
    updating = true;
    updateError = '';
    try {
      toolsNow = await api.toolsUpdate();
      ontools(toolsNow);
    } catch (e) {
      updateError = toUserError(e).detail;
    } finally {
      updating = false;
    }
  }

  function save() {
    onsave({ ...draft, maxParallel: Math.min(6, Math.max(1, Math.round(Number(draft.maxParallel)) || 2)) });
  }

  const toolLine = (name: string, path: string | null) =>
    `${name}: ${path ? $tr('settings.installed') : $tr('settings.notInstalled')}`;
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="overlay">
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="{uid}-title">
    <h2 id="{uid}-title">{$tr('settings')}</h2>

    <div class="row">
      <span class="label">{$tr('settings.folder')}</span>
      <span class="value">{draft.outputDir ?? $tr('settings.defaultFolder')}</span>
      <button type="button" onclick={chooseFolder}>{$tr('settings.chooseFolder')}</button>
      <button type="button" onclick={() => (draft.outputDir = null)}>{$tr('settings.resetFolder')}</button>
    </div>

    <div class="row">
      <label for="{uid}-container">{$tr('settings.container')}</label>
      <select id="{uid}-container" bind:value={draft.container}>
        <option value="mp4">MP4</option>
        <option value="mkv">MKV</option>
      </select>
    </div>

    <div class="row">
      <label for="{uid}-parallel">{$tr('settings.parallel')}</label>
      <input id="{uid}-parallel" type="number" min="1" max="6" bind:value={draft.maxParallel} />
    </div>

    <div class="row">
      <label for="{uid}-cookies">{$tr('settings.cookies')}</label>
      <select id="{uid}-cookies" bind:value={draft.cookiesFrom}>
        <option value="none">{$tr('settings.cookiesNone')}</option>
        <option value="safari">Safari</option>
        <option value="chrome">Chrome</option>
        <option value="firefox">Firefox</option>
      </select>
    </div>

    <div class="row">
      <label for="{uid}-language">{$tr('settings.language')}</label>
      <select id="{uid}-language" bind:value={draft.language}>
        <option value="auto">{$tr('settings.auto')}</option>
        <option value="uk">Українська</option>
        <option value="en">English</option>
      </select>
    </div>

    <div class="row">
      <label for="{uid}-theme">{$tr('settings.theme')}</label>
      <select id="{uid}-theme" bind:value={draft.theme}>
        <option value="auto">{$tr('settings.auto')}</option>
        <option value="light">{$tr('settings.light')}</option>
        <option value="dark">{$tr('settings.dark')}</option>
      </select>
    </div>

    <div class="tools">
      <span class="label">{$tr('settings.tools')}</span>
      <ul>
        <li>{toolLine('yt-dlp', toolsNow.ytdlp)}</li>
        <li>{toolLine('ffmpeg', toolsNow.ffmpeg)}</li>
      </ul>
      <button type="button" disabled={updating} onclick={updateYtdlp}>
        {updating ? $tr('settings.updating') : $tr('settings.updateYtdlp')}
      </button>
      {#if updateError}<p class="error" role="alert">{updateError}</p>{/if}
    </div>

    <p class="fair">{$tr('settings.fairUse')}</p>

    <div class="buttons">
      <button type="button" onclick={onclose}>{$tr('settings.close')}</button>
      <button type="button" class="primary" onclick={save}>{$tr('settings.save')}</button>
    </div>
  </div>
</div>
