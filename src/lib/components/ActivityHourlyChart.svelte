<script>
  import { locale, formatDurationLocalized, t } from '$lib/i18n/index.js';

  export let data = [];
  export let distributionTitle = '';
  export let distributionSubtitleKey = 'hourlyChart.distributionSubtitle';
  export let mode = 'column';
  export let peakHourLabel = '';
  export let peakDurationLabel = '';

  const keyHours = [0, 6, 12, 18, 23];
  let selectedHour = null;
  $: currentLocale = $locale;

  function formatHourLabel(hour) {
    return `${String(hour).padStart(2, '0')}:00`;
  }

  function formatHourRangeLabel(hour) {
    return `${formatHourLabel(hour)} - ${formatHourLabel((hour + 1) % 24)}`;
  }

  function showHourLabel(hour) {
    return keyHours.includes(hour);
  }

  function hourAxisLabelAlignmentClass(hour) {
    if (hour === 0) {
      return 'justify-start';
    }
    if (hour === 23) {
      return 'justify-end';
    }
    return 'justify-center';
  }

  function tooltipAlignmentClass(hour) {
    if (hour <= 2) {
      return 'left-0';
    }
    if (hour >= 21) {
      return 'right-0';
    }
    return 'left-1/2 -translate-x-1/2';
  }

  function formatAxisTickLabel(seconds) {
    const minutes = Math.round(seconds / 60);
    if (minutes === 0) {
      return '0';
    }
    if (minutes % 60 === 0) {
      return `${minutes / 60}h`;
    }
    return `${minutes}m`;
  }

  function selectHour(hour) {
    selectedHour = hour;
  }

  $: buckets = Array.from({ length: 24 }, (_, hour) => {
    const existing = data.find((item) => item.hour === hour);
    return existing || { hour, duration: 0 };
  });

  $: maxDuration = Math.max(1, ...buckets.map((bucket) => bucket.duration || 0));
  $: activeBuckets = buckets.filter((bucket) => bucket.duration > 0);
  $: totalDuration = buckets.reduce((sum, bucket) => sum + (bucket.duration || 0), 0);
  $: peakBucket = buckets.reduce(
    (peak, bucket) => (bucket.duration > peak.duration ? bucket : peak),
    buckets[0] || { hour: 0, duration: 0 }
  );
  $: topBuckets = [...activeBuckets]
    .sort((left, right) => right.duration - left.duration || left.hour - right.hour)
    .slice(0, 3);
  $: selectedBucket = buckets[selectedHour] || null;
  $: axisMax = (() => {
    const raw = Math.max(maxDuration, 60);
    const minute = 60;
    const candidates = [5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 240, 300, 360, 480, 720]
      .map((value) => value * minute);
    return candidates.find((candidate) => candidate >= raw) || Math.ceil(raw / 3600) * 3600;
  })();
  $: yAxisTicks = [axisMax, Math.round(axisMax * 2 / 3), Math.round(axisMax / 3), 0];
</script>

<div class="space-y-4" data-locale={currentLocale}>
  <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
    <div class="min-h-[104px] rounded-2xl border border-slate-100 bg-white p-4 dark:border-slate-700/60 dark:bg-slate-800/80">
      <p class="text-[13px] font-medium text-slate-400 dark:text-slate-500">{peakHourLabel || t('hourlyChart.peakHour')}</p>
      <p class="mt-5 text-[1.8rem] font-semibold tracking-tight text-slate-800 dark:text-white">
        {formatHourLabel(peakBucket.hour)}
      </p>
    </div>
    <div class="min-h-[104px] rounded-2xl border border-slate-100 bg-white p-4 dark:border-slate-700/60 dark:bg-slate-800/80">
      <p class="text-[13px] font-medium text-slate-400 dark:text-slate-500">{peakDurationLabel || t('hourlyChart.peakDuration')}</p>
      <p class="mt-5 text-[1.8rem] font-semibold tracking-tight text-slate-800 dark:text-white">
        {formatDurationLocalized(peakBucket.duration, { compact: true })}
      </p>
    </div>
    <div class="min-h-[104px] rounded-2xl border border-slate-100 bg-white p-4 dark:border-slate-700/60 dark:bg-slate-800/80">
      <p class="text-[13px] font-medium text-slate-400 dark:text-slate-500">{t('hourlyChart.activeHours')}</p>
      <p class="mt-5 text-[1.8rem] font-semibold tracking-tight text-slate-800 dark:text-white">
        {activeBuckets.length}
      </p>
    </div>
    <div class="min-h-[104px] rounded-2xl border border-slate-100 bg-white p-4 dark:border-slate-700/60 dark:bg-slate-800/80">
      <p class="text-[13px] font-medium text-slate-400 dark:text-slate-500">{t('hourlyChart.totalDuration')}</p>
      <p class="mt-5 text-[1.8rem] font-semibold tracking-tight text-slate-800 dark:text-white">
        {formatDurationLocalized(totalDuration, { compact: true })}
      </p>
    </div>
  </div>

  <div class="rounded-2xl border border-slate-100 bg-white p-4 dark:border-slate-700/60 dark:bg-slate-800/80">
    <div class="mb-4 flex items-center justify-between gap-3">
      <div>
        <p class="text-sm font-semibold text-slate-700 dark:text-slate-200">
          {distributionTitle || t('hourlyChart.distributionTitle')}
        </p>
        <p class="mt-1 text-xs text-slate-500 dark:text-slate-400">
          {t(distributionSubtitleKey, {
            hour: formatHourLabel(peakBucket.hour),
            duration: formatDurationLocalized(peakBucket.duration),
          })}
        </p>
      </div>
      {#if topBuckets.length > 0}
        <div class="hidden items-center gap-2 lg:flex">
          {#each topBuckets as bucket, index}
            <span class="rounded-full bg-slate-100 px-2.5 py-1 text-xs text-slate-500 dark:bg-slate-700/70 dark:text-slate-300">
              {t('hourlyChart.topHour', { index: index + 1, hour: formatHourLabel(bucket.hour) })}
            </span>
          {/each}
        </div>
      {/if}
    </div>

    {#if mode === 'row'}
      <div class="space-y-2 rounded-2xl bg-slate-50 p-3 dark:bg-slate-900/40">
        {#each buckets as bucket}
          {@const width = bucket.duration > 0 ? Math.max((bucket.duration / maxDuration) * 100, 3) : 1}
          {@const isPeak = bucket.duration > 0 && bucket.hour === peakBucket.hour}
          <button
            type="button"
            class={`grid w-full grid-cols-[3.25rem_minmax(0,1fr)_4.75rem] items-center gap-2 rounded-xl px-1.5 py-1 text-left transition-colors duration-200 ${selectedHour === bucket.hour ? 'bg-sky-50 dark:bg-sky-500/10' : 'hover:bg-white/70 dark:hover:bg-slate-800/60'}`}
            aria-pressed={selectedHour === bucket.hour}
            on:click={() => selectHour(bucket.hour)}
          >
            <span class="text-[11px] font-medium text-slate-500 dark:text-slate-400">{formatHourLabel(bucket.hour)}</span>
            <div class="h-3 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-700/60">
              <div
                class={`h-full rounded-full transition-all duration-300 ${isPeak ? 'bg-sky-500 dark:bg-sky-400' : 'bg-slate-400 dark:bg-slate-500'}`}
                style={`width: ${width}%; opacity: ${bucket.duration > 0 ? 1 : 0.35};`}
                title={`${formatHourRangeLabel(bucket.hour)} · ${formatDurationLocalized(bucket.duration)}`}
              ></div>
            </div>
            <span class="text-right text-[11px] font-medium tabular-nums text-slate-500 dark:text-slate-400">{formatDurationLocalized(bucket.duration, { compact: true })}</span>
          </button>
        {/each}
      </div>
    {:else}
      <div class="overflow-hidden rounded-2xl bg-slate-50 px-3 pb-3 pt-4 dark:bg-slate-900/40">
        <div class="grid grid-cols-[2.9rem_minmax(0,1fr)] gap-2">
          <div class="relative h-44">
            {#each yAxisTicks as tick, index}
              <div class="absolute inset-x-0 flex -translate-y-1/2 items-center justify-end text-[10px] font-medium text-slate-400 dark:text-slate-500" style={`top: ${(index / 3) * 100}%`}>
                <span class="whitespace-nowrap">{formatAxisTickLabel(tick)}</span>
              </div>
            {/each}
          </div>
          <div class="relative">
            <div class="pointer-events-none absolute inset-x-0 top-0 bottom-8">
              <div class="absolute inset-x-0 top-0 border-t border-dashed border-slate-200 dark:border-slate-700/80"></div>
              <div class="absolute inset-x-0 top-1/3 border-t border-dashed border-slate-200 dark:border-slate-700/80"></div>
              <div class="absolute inset-x-0 top-2/3 border-t border-dashed border-slate-200 dark:border-slate-700/80"></div>
              <div class="absolute inset-x-0 bottom-0 border-t border-dashed border-slate-200 dark:border-slate-700/80"></div>
            </div>

            <div class="relative flex h-44 items-end gap-1">
              {#each buckets as bucket}
                {@const height = bucket.duration > 0 ? Math.max((bucket.duration / axisMax) * 100, 6) : 2}
                {@const isPeak = bucket.duration > 0 && bucket.hour === peakBucket.hour}
                <div class="relative flex h-full min-w-0 flex-1 flex-col justify-end">
                  {#if mode === 'column' && selectedHour === bucket.hour}
                    <div class={`pointer-events-none absolute top-1 z-10 ${tooltipAlignmentClass(bucket.hour)}`}>
                      <span class="flex min-w-[6.75rem] flex-col items-center rounded-2xl bg-slate-900 px-2.5 py-1.5 text-[10px] font-medium text-white shadow-sm dark:bg-slate-100 dark:text-slate-900">
                        <span class="whitespace-nowrap">
                          {formatHourRangeLabel(bucket.hour)}
                        </span>
                        <span class="mt-0.5 whitespace-nowrap text-[9px] opacity-80">
                          {formatDurationLocalized(bucket.duration, { compact: true })}
                        </span>
                      </span>
                    </div>
                  {/if}
                  <button
                    type="button"
                    class={`w-full rounded-t-[10px] transition-all duration-300 focus:outline-none focus:ring-2 focus:ring-sky-300 dark:focus:ring-sky-500 ${selectedHour === bucket.hour ? 'ring-2 ring-sky-300 dark:ring-sky-500' : ''} ${isPeak ? 'bg-sky-500 dark:bg-sky-400' : 'bg-slate-300 dark:bg-slate-600'}`}
                    style={`height: ${height}%; opacity: ${bucket.duration > 0 ? 1 : 0.35};`}
                    title={`${formatHourRangeLabel(bucket.hour)} · ${formatDurationLocalized(bucket.duration)}`}
                    aria-pressed={selectedHour === bucket.hour}
                    on:click={() => selectHour(bucket.hour)}
                  ></button>
                </div>
              {/each}
            </div>

            <div class="mt-3 flex gap-1 px-1">
              {#each buckets as bucket}
                <div class="flex-1">
                  <div class={`flex w-full ${hourAxisLabelAlignmentClass(bucket.hour)}`}>
                  <span class={`text-[10px] font-medium ${showHourLabel(bucket.hour) ? 'text-slate-400 dark:text-slate-500' : 'text-transparent'}`}>
                    {showHourLabel(bucket.hour) ? formatHourLabel(bucket.hour) : '.'}
                  </span>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        </div>
      </div>

      {#if mode === 'column' && selectedBucket}
        <div class="mt-3 grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 rounded-2xl bg-sky-50 px-3.5 py-3 text-left dark:bg-sky-500/10">
          <span class="inline-flex items-center rounded-full bg-white px-2.5 py-1 text-[11px] font-semibold tracking-[0.08em] text-sky-700 shadow-[inset_0_1px_0_rgba(255,255,255,0.8)] dark:bg-slate-900/80 dark:text-sky-300">
            当前选中
          </span>
          <span class="min-w-0 truncate text-sm font-medium text-slate-700 dark:text-slate-200">
            {formatHourRangeLabel(selectedBucket.hour)}
          </span>
          <span class="text-sm font-semibold tabular-nums text-slate-500 dark:text-slate-300">
            {formatDurationLocalized(selectedBucket.duration, { compact: true })}
          </span>
        </div>
      {/if}
    {/if}
  </div>
</div>
