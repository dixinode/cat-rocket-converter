<!-- Page 5 configures batch rename patterns and previews the first filenames. -->
<script>
  import { wizard, updateWizard } from '../stores/wizard.js';

  function pad(value, length = 3) {
    return String(value).padStart(length, '0');
  }

  function extension() {
    return $wizard.outputFormat === 'jpeg' ? 'jpg' : $wizard.outputFormat;
  }

  function getBaseName(path) {
    const filename = path.split(/[\\/]/).pop() ?? path;
    return filename.replace(/\.[^.]+$/, '');
  }

  function dateStamp() {
    const now = new Date();
    const yyyy = now.getFullYear();
    const mm = pad(now.getMonth() + 1, 2);
    const dd = pad(now.getDate(), 2);
    const hh = pad(now.getHours(), 2);
    const min = pad(now.getMinutes(), 2);
    const sec = pad(now.getSeconds(), 2);
    return `${yyyy}${mm}${dd}_${hh}${min}${sec}`;
  }

  function previewName(path, index) {
    const ext = extension();

    if ($wizard.rename.pattern === 'datetime') {
      return `${$wizard.rename.prefix}${dateStamp()}_${pad(index + 1)}.${ext}`;
    }

    if ($wizard.rename.pattern === 'suffix') {
      return `${getBaseName(path)}${$wizard.rename.suffix}.${ext}`;
    }

    if ($wizard.rename.pattern === 'sequential') {
      return `${$wizard.rename.seqBase}${pad($wizard.rename.seqStart + index)}.${ext}`;
    }

    return `${getBaseName(path)}.${ext}`;
  }

  function updatePattern(pattern) {
    updateWizard((state) => ({ ...state, rename: { ...state.rename, pattern } }));
  }

  function updateField(field, value) {
    updateWizard((state) => ({ ...state, rename: { ...state.rename, [field]: value } }));
  }

  $: previewFiles = $wizard.files.slice(0, 3).map((path, index) => previewName(path, index));
</script>

<section class="page-shell">
  <div class="rename-panel">
    <p class="page-title">Rename files</p>

    <label class="option-row">
      <input checked={$wizard.rename.pattern === 'datetime'} name="rename-pattern" type="radio" on:change={() => updatePattern('datetime')} />
      <span>Date + Time prefix</span>
    </label>
    {#if $wizard.rename.pattern === 'datetime'}
      <input class="text-input wide-input" type="text" value={$wizard.rename.prefix} on:input={(event) => updateField('prefix', event.currentTarget.value)} />
    {/if}

    <label class="option-row">
      <input checked={$wizard.rename.pattern === 'suffix'} name="rename-pattern" type="radio" on:change={() => updatePattern('suffix')} />
      <span>Original name + suffix</span>
    </label>
    {#if $wizard.rename.pattern === 'suffix'}
      <input class="text-input wide-input" type="text" value={$wizard.rename.suffix} on:input={(event) => updateField('suffix', event.currentTarget.value)} />
    {/if}

    <label class="option-row">
      <input checked={$wizard.rename.pattern === 'sequential'} name="rename-pattern" type="radio" on:change={() => updatePattern('sequential')} />
      <span>Sequential</span>
    </label>
    {#if $wizard.rename.pattern === 'sequential'}
      <div class="seq-row">
        <input class="text-input seq-base" type="text" value={$wizard.rename.seqBase} on:input={(event) => updateField('seqBase', event.currentTarget.value)} />
        <input class="text-input seq-start" min="1" type="number" value={$wizard.rename.seqStart} on:input={(event) => updateField('seqStart', Number(event.currentTarget.value) || 1)} />
      </div>
    {/if}

    <label class="option-row">
      <input checked={$wizard.rename.pattern === 'keep'} name="rename-pattern" type="radio" on:change={() => updatePattern('keep')} />
      <span>Keep original names</span>
    </label>

    <div class="preview-list">
      {#each previewFiles as file}
        <div class="preview-item">{file}</div>
      {/each}
    </div>
  </div>
</section>

<style>
  .rename-panel {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 2px 18px 0;
    box-sizing: border-box;
  }

  .option-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-primary);
  }

  .wide-input {
    width: 190px;
    height: 30px;
    margin-left: 22px;
    text-align: left;
    padding: 0 10px;
  }

  .seq-row {
    display: flex;
    gap: 8px;
    margin-left: 22px;
  }

  .seq-base {
    width: 142px;
    height: 30px;
    text-align: left;
    padding: 0 10px;
  }

  .seq-start {
    width: 56px;
    height: 30px;
  }

  .preview-list {
    width: 100%;
    margin-top: 4px;
    padding-top: 6px;
    border-top: 1px solid var(--divider);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .preview-item {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
