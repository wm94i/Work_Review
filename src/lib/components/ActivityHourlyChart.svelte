<script>
  import { locale, formatDurationLocalized, t } from '$lib/i18n/index.js';

  export let data = [];

  const keyHours = [0, 6, 12, 18, 23];
  $: currentLocale = $locale;

  function formatHourLabel(hour) {
    return `${String(hour).padStart(2, '0')}:00`;
  }

  function showHourLabel(hour) {
    return keyHours.includes(hour);
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
</script>

<div class="space-y-4" data-locale={currentLocale}>
  <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
    <div class="min-h-[104px] rounded-2xl border border-slate-100 bg-white p-4 dark:border-slate-700/60 dark:bg-slate-800/80">
      <p class="text-[13px] font-medium text-slate-400 dark:text-slate-500">{t('hourlyChart.peakHour')}</p>
      <p class="mt-5 text-[1.8rem] font-semibold tracking-tight text-slate-800 dark:text-white">
        {formatHourLabel(peakBucket.hour)}
      </p>
    </div>
    <div class="min-h-[104px] rounded-2xl border border-slate-100 bg-white p-4 dark:border-slate-700/60 dark:bg-slate-800/80">
      <p class="text-[13px] font-medium text-slate-400 dark:text-slate-500">{t('hourlyChart.peakDuration')}</p>
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
        <p class="text-sm font-semibold text-slate-700 dark:text-slate-200">{t('hourlyChart.distributionTitle')}</p>
        <p class="mt-1 text-xs text-slate-500 dark:text-slate-400">
          {t('hourlyChart.distributionSubtitle', {
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

    <div class="relative rounded-2xl bg-slate-50 px-3 pb-3 pt-4 dark:bg-slate-900/40">
      <div class="pointer-events-none absolute inset-x-3 top-4 bottom-8">
        <div class="absolute inset-x-0 top-0 border-t border-dashed border-slate-200 dark:border-slate-700/80"></div>
        <div class="absolute inset-x-0 top-1/2 border-t border-dashed border-slate-200 dark:border-slate-700/80"></div>
        <div class="absolute inset-x-0 bottom-0 border-t border-dashed border-slate-200 dark:border-slate-700/80"></div>
      </div>

      <div class="relative flex h-44 items-end gap-1">
        {#each buckets as bucket}
          {@const height = bucket.duration > 0 ? Math.max((bucket.duration / maxDuration) * 100, 6) : 2}
          {@const isPeak = bucket.duration > 0 && bucket.hour === peakBucket.hour}
          <div class="flex h-full min-w-0 flex-1 flex-col justify-end">
            <div
              class={`w-full rounded-t-[10px] transition-all duration-300 ${isPeak ? 'bg-sky-500 dark:bg-sky-400' : 'bg-slate-300 dark:bg-slate-600'}`}
              style={`height: ${height}%; opacity: ${bucket.duration > 0 ? 1 : 0.35};`}
              title={`${formatHourLabel(bucket.hour)} · ${formatDurationLocalized(bucket.duration)}`}
            ></div>
          </div>
        {/each}
      </div>

      <div class="mt-3 flex gap-1">
        {#each buckets as bucket}
          <div class="flex-1 text-center">
            <span class={`text-[10px] font-medium ${showHourLabel(bucket.hour) ? 'text-slate-400 dark:text-slate-500' : 'text-transparent'}`}>
              {showHourLabel(bucket.hour) ? formatHourLabel(bucket.hour) : '.'}
            </span>
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>
