<script>
  import { toast, clearToast } from '$lib/stores/toast.js';

  const iconMap = {
    success: 'M5 13l4 4L19 7',
    error: 'M6 18L18 6M6 6l12 12',
    warning: 'M12 9v4m0 4h.01M10.29 3.86l-7.5 13A1 1 0 003.65 18h16.7a1 1 0 00.86-1.5l-7.5-13a1 1 0 00-1.72 0z',
    info: 'M13 16h-1v-4h-1m1-4h.01M12 22a10 10 0 110-20 10 10 0 010 20z',
  };

  const colorMap = {
    success: 'bg-slate-800 dark:bg-slate-200 text-white dark:text-slate-800',
    error: 'bg-red-600 text-white',
    warning: 'bg-amber-500 text-white',
    info: 'bg-sky-600 text-white',
  };

  $: toastState = $toast;
  $: iconPath = iconMap[toastState?.type] || iconMap.info;
  $: toastClass = colorMap[toastState?.type] || colorMap.info;
</script>

{#if toastState}
  <div class="fixed bottom-6 left-1/2 -translate-x-1/2 z-[100] animate-fadeIn">
    <button
      type="button"
      on:click={clearToast}
      class={`px-4 py-2.5 rounded-xl shadow-lg text-sm font-medium flex items-center gap-2 ${toastClass}`}
    >
      <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={iconPath} />
      </svg>
      <span>{toastState.message}</span>
    </button>
  </div>
{/if}
