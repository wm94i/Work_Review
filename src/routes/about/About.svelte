<script>
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-shell';
  import { getVersion } from '@tauri-apps/api/app';
  import { locale, t } from '$lib/i18n/index.js';
  import { runUpdateFlow } from '$lib/utils/updater.js';

  const wechatSponsorshipQr = new URL('../../../docs/sponsorship/vx.png', import.meta.url).href;
  const alipaySponsorshipQr = new URL('../../../docs/sponsorship/zfb.png', import.meta.url).href;

  let appVersion = '';
  let isCheckingUpdate = false;
  let isSponsorshipOpen = false;
  let updateStatus = '';
  let updateStatusTimer = null;
  $: currentLocale = $locale;

  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch (e) {
      console.error('初始化失败:', e);
      appVersion = '1.0.0';
    }
  });

  async function openGitHub() {
    await open('https://github.com/wm94i/Work_Review');
  }

  async function openDataDir() {
    try {
      await invoke('open_data_dir');
    } catch (e) {
      console.error('打开目录失败:', e);
    }
  }

  function openSponsorshipModal() {
    isSponsorshipOpen = true;
  }

  function closeSponsorshipModal() {
    isSponsorshipOpen = false;
  }

  async function checkForUpdates() {
    if (isCheckingUpdate) return;

    isCheckingUpdate = true;
    updateStatus = t('about.checkingUpdates');

    await runUpdateFlow({
      onStatusChange: (status) => {
        updateStatus = status;
      },
    });

    isCheckingUpdate = false;
    if (updateStatus) {
      clearTimeout(updateStatusTimer);
      updateStatusTimer = setTimeout(() => {
        updateStatus = '';
        updateStatusTimer = null;
      }, 3000);
    }
  }

  onDestroy(() => {
    clearTimeout(updateStatusTimer);
  });

  function handleWindowKeydown(event) {
    if (event.key === 'Escape' && isSponsorshipOpen) {
      closeSponsorshipModal();
    }
  }
</script>

<svelte:window on:keydown={handleWindowKeydown} />

<div class="page-shell about-editorial-shell" data-locale={currentLocale}>
  <div class="mx-auto w-full max-w-4xl about-minimal-shell">
    <section class="page-card about-brand-card">
      <div class="about-brand-head">
        <div class="about-brand-mark">
          <img src="/icons/256x256.png" alt="Work Review" class="h-16 w-16 rounded-[18px] object-cover" />
        </div>
        <span class="page-inline-chip-brand">v{appVersion}</span>
      </div>

      <div class="about-brand-copy">
        <h1 class="about-brand-title">Work Review</h1>
        <p class="about-brand-description">{t('about.description')}</p>
      </div>

      <div class="about-action-strip">
        <div class="about-action-row">
          <button on:click={openGitHub} class="page-action-secondary min-h-10 px-4 py-2">
            <svg class="w-4 h-4 shrink-0" fill="currentColor" viewBox="0 0 24 24"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>
            <span class="leading-none">GitHub</span>
          </button>
          <button on:click={openDataDir} class="page-action-secondary min-h-10 px-4 py-2">
            <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>
            <span class="leading-none">{t('about.openDataDir')}</span>
          </button>
          <button
            type="button"
            on:click={openSponsorshipModal}
            class="about-support-link"
          >
            <svg class="w-4 h-4 shrink-0 text-rose-500 dark:text-rose-400" fill="currentColor" viewBox="0 0 24 24">
              <path d="M11.996 21.357c-.34 0-.673-.092-.966-.267C8.304 19.466 2.25 15.48 2.25 9.806c0-3.034 2.395-5.556 5.47-5.556 1.708 0 3.31.78 4.276 2.074.966-1.293 2.567-2.074 4.275-2.074 3.074 0 5.48 2.522 5.48 5.556 0 5.674-6.054 9.66-8.78 11.284a1.88 1.88 0 0 1-.975.267Z" />
            </svg>
            <span class="leading-none">{t('about.sponsorship')}</span>
          </button>
        </div>

        <div class="flex justify-center">
          <button
            on:click={checkForUpdates}
            disabled={isCheckingUpdate}
            class="page-action-brand min-h-10 px-5 py-2 disabled:cursor-wait"
          >
            {#if isCheckingUpdate}
              <svg class="animate-spin h-4 w-4 shrink-0 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <span class="leading-none">{t('about.checkingUpdates')}</span>
            {:else}
              <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>
              <span class="leading-none">{t('about.checkUpdates')}</span>
            {/if}
          </button>
        </div>
      </div>
    </section>

    <section class="about-trust-grid">
      <article class="page-card about-trust-card">
        <span class="about-trust-kicker">01</span>
        <h3 class="about-trust-title">{t('about.localFirstTitle')}</h3>
        <p class="about-trust-copy">{t('about.localFirstCopy')}</p>
      </article>
      <article class="page-card about-trust-card">
        <span class="about-trust-kicker">02</span>
        <h3 class="about-trust-title">{t('about.timelineTrustTitle')}</h3>
        <p class="about-trust-copy">{t('about.timelineTrustCopy')}</p>
      </article>
      <article class="page-card about-trust-card">
        <span class="about-trust-kicker">03</span>
        <h3 class="about-trust-title">{t('about.reportTrustTitle')}</h3>
        <p class="about-trust-copy">{t('about.reportTrustCopy')}</p>
      </article>
    </section>

    <section class="about-tech-stack">
      <span class="about-tech-pill about-tech-pill-primary">Tauri 2</span>
      <span class="about-tech-pill">Svelte</span>
      <span class="about-tech-pill">Rust</span>
      <span class="about-tech-pill">SQLite</span>
    </section>

    {#if updateStatus}
      <div class="page-banner-warning justify-center text-center">
        <div>
          <p class="font-semibold">{t('about.updateStatus')}</p>
          <p class="text-sm mt-1">{updateStatus}</p>
        </div>
      </div>
    {/if}
  </div>
</div>

{#if isSponsorshipOpen}
  <div
    class="fixed inset-0 z-[135] flex items-center justify-center bg-slate-950/52 px-4 py-6 backdrop-blur-md animate-fadeIn"
  >
    <button
      type="button"
      class="absolute inset-0 cursor-default"
      on:click={closeSponsorshipModal}
      aria-label={t('about.closeSupportDialog')}
    ></button>

    <div
      class="relative z-10 w-full max-w-3xl rounded-[32px] border border-slate-200/80 bg-white p-6 text-left shadow-2xl shadow-slate-950/20 dark:border-slate-700/70 dark:bg-slate-900 sm:p-7"
      role="dialog"
      aria-modal="true"
      aria-labelledby="sponsorship-dialog-title"
    >
      <div class="flex items-start justify-between gap-4">
        <div class="min-w-0">
          <div class="inline-flex items-center gap-2 rounded-full border border-amber-200 bg-amber-50 px-3 py-1 text-[11px] font-semibold tracking-[0.14em] text-amber-700 dark:border-amber-900/60 dark:bg-amber-950/40 dark:text-amber-300">
            {t('about.supportBadge')}
          </div>
          <h3 id="sponsorship-dialog-title" class="mt-3 text-2xl font-semibold tracking-tight text-slate-900 dark:text-white">
            {t('about.supportTitle')}
          </h3>
          <p class="mt-3 max-w-2xl text-sm leading-7 text-slate-600 dark:text-slate-300">
            {t('about.supportCopy')}
          </p>
          <p class="text-sm leading-7 text-slate-500 dark:text-slate-400">
            {t('about.supportCopy2')}
          </p>
        </div>

        <button
          type="button"
          on:click={closeSponsorshipModal}
          class="inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl border border-slate-200 bg-white text-slate-500 transition hover:bg-slate-50 hover:text-slate-700 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-400 dark:hover:bg-slate-700 dark:hover:text-slate-200"
          aria-label={t('about.closeSupportDialog')}
        >
          <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="m6 6 12 12M18 6 6 18" />
          </svg>
        </button>
      </div>

      <div class="mt-6 grid gap-4 md:grid-cols-2">
        <div class="rounded-[28px] border border-slate-200/80 bg-slate-50/80 p-4 dark:border-slate-700/80 dark:bg-slate-800/50">
          <div class="flex items-center gap-2">
            <div class="flex h-10 w-10 items-center justify-center rounded-2xl bg-emerald-100 text-emerald-700 dark:bg-emerald-950/50 dark:text-emerald-300">
              <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M7.5 7.5h9v9h-9z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4.5 12h3m9 0h3M12 4.5v3m0 9v3" />
              </svg>
            </div>
            <div>
              <h4 class="text-base font-semibold text-slate-900 dark:text-white">{t('about.wechat')}</h4>
            </div>
          </div>
          <div class="mt-4 rounded-[24px] bg-white p-4 shadow-[inset_0_1px_0_rgba(255,255,255,0.8)] dark:bg-slate-950">
            <img src={wechatSponsorshipQr} alt={t('about.wechatQrAlt')} class="mx-auto aspect-square w-full max-w-[220px] rounded-2xl object-contain" />
          </div>
        </div>

        <div class="rounded-[28px] border border-slate-200/80 bg-slate-50/80 p-4 dark:border-slate-700/80 dark:bg-slate-800/50">
          <div class="flex items-center gap-2">
            <div class="flex h-10 w-10 items-center justify-center rounded-2xl bg-sky-100 text-sky-700 dark:bg-sky-950/50 dark:text-sky-300">
              <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4.5 8.25h15M6.75 4.5h10.5A2.25 2.25 0 0 1 19.5 6.75v10.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 17.25V6.75A2.25 2.25 0 0 1 6.75 4.5Z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M8.25 13.5h7.5" />
              </svg>
            </div>
            <div>
              <h4 class="text-base font-semibold text-slate-900 dark:text-white">{t('about.alipay')}</h4>
            </div>
          </div>
          <div class="mt-4 rounded-[24px] bg-white p-4 shadow-[inset_0_1px_0_rgba(255,255,255,0.8)] dark:bg-slate-950">
            <img src={alipaySponsorshipQr} alt={t('about.alipayQrAlt')} class="mx-auto aspect-square w-full max-w-[220px] rounded-2xl object-contain" />
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}
