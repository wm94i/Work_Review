<script>
  export let followup = null;
  export let copy = null;
  export let onTimeline = () => {};
  export let onFocus = () => {};
  export let onRemember = () => {};
  export let onSnooze = () => {};
  export let onDismiss = () => {};

  const CLAMP_STYLE_TWO_LINES =
    'display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; overflow: hidden;';
  const CLAMP_STYLE_FOUR_LINES =
    'display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 4; overflow: hidden;';
</script>

{#if followup && copy}
  <div class="absolute inset-0 z-30 overflow-visible pointer-events-none">
    <section
      class={`pointer-events-auto absolute right-[4%] top-[8px] flex w-[min(92vw,348px)] max-w-[348px] flex-col overflow-hidden rounded-[16px] border border-slate-200/95 bg-[rgba(255,255,255,0.96)] p-[14px] text-slate-900 shadow-[0_14px_28px_rgba(15,23,42,0.14),0_4px_12px_rgba(15,23,42,0.07)] backdrop-blur-[14px] ${copy.surfaceClass || ''}`}
      style="max-height: calc(100vh - 16px);"
    >
      <div class="pointer-events-none absolute inset-[1px] rounded-[15px] border border-white/70"></div>

      <button
        type="button"
        class="absolute right-2 top-2 z-10 inline-flex h-5 w-5 items-center justify-center rounded-full text-slate-400 transition hover:bg-slate-900/6 hover:text-slate-700"
        aria-label={copy.dismissLabel}
        on:click={onDismiss}
      >
        ×
      </button>

      <div class="relative shrink-0 grid grid-cols-[auto,minmax(0,1fr)] items-start gap-2 pr-7">
        <span class={`inline-flex shrink-0 whitespace-nowrap rounded-full px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.08em] ${copy.badgeClass || 'bg-emerald-500/12 text-emerald-700'}`}>
          {copy.personaLabel}
        </span>
        <span
          class="min-w-0 text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-500"
          style={CLAMP_STYLE_TWO_LINES}
        >
          {copy.title}
        </span>
      </div>

      <div class="relative mt-2 min-h-0 flex-1 overflow-y-auto pr-1">
        <p
          class="break-words text-[13px] font-semibold leading-[1.45] text-slate-900"
          style={CLAMP_STYLE_FOUR_LINES}
        >
          {copy.summary}
        </p>
        <p class="mt-1 break-words text-[11px] leading-[1.45] text-slate-500">
          {copy.meta}
        </p>
        <p
          class="mt-1 break-words pb-1 text-[11px] leading-[1.45] text-slate-600"
          style={CLAMP_STYLE_TWO_LINES}
        >
          {copy.strategy}
        </p>
      </div>

      <div class="relative mt-3 shrink-0 space-y-2">
        <button
          type="button"
          class={`inline-flex w-full items-center justify-center rounded-[10px] px-3 py-2 text-[12px] font-semibold transition ${copy.primaryClass || 'bg-emerald-500 hover:bg-emerald-600 text-white'}`}
          on:click={onTimeline}
        >
          {copy.openTimeline}
        </button>

        <div class="flex flex-col gap-2">
          <button
            type="button"
            class="inline-flex min-h-[34px] w-full items-center justify-center gap-1 rounded-[10px] border border-slate-200 bg-white px-3 py-2 text-[12px] font-medium text-slate-700 transition hover:border-slate-300 hover:bg-slate-50"
            title={copy.focusFull || copy.focus}
            aria-label={copy.focusFull || copy.focus}
            on:click={onFocus}
          >
            <span class="truncate">{copy.focusFull || copy.focus}</span>
          </button>
          <button
            type="button"
            class="inline-flex min-h-[34px] w-full items-center justify-center gap-1 rounded-[10px] border border-slate-200 bg-white px-3 py-2 text-[12px] font-medium text-slate-700 transition hover:border-slate-300 hover:bg-slate-50"
            title={copy.rememberFull || copy.remember}
            aria-label={copy.rememberFull || copy.remember}
            on:click={onRemember}
          >
            <span class="truncate">{copy.rememberFull || copy.remember}</span>
          </button>
          <button
            type="button"
            class="inline-flex min-h-[34px] w-full items-center justify-center gap-1 rounded-[10px] border border-slate-200 bg-white px-3 py-2 text-[12px] font-medium text-slate-700 transition hover:border-slate-300 hover:bg-slate-50"
            title={copy.snoozeFull || copy.snooze}
            aria-label={copy.snoozeFull || copy.snooze}
            on:click={onSnooze}
          >
            <span class="truncate">{copy.snoozeFull || copy.snooze}</span>
          </button>
        </div>
      </div>

      <div class="followup-tail absolute left-5 top-[calc(100%-6px)] h-3 w-3 rotate-45 rounded-[3px] border border-slate-200/95 bg-[rgba(255,255,255,0.96)] shadow-[0_6px_16px_rgba(15,23,42,0.06)]"></div>
    </section>
  </div>
{/if}
