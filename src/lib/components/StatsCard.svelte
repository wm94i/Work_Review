<script>
  export let title = '';
  export let value = '';
  export let icon = '';
  export let color = 'indigo';
  export let href = null;

  const iconColors = {
    indigo: 'text-indigo-500 dark:text-indigo-400',
    emerald: 'text-emerald-500 dark:text-emerald-400',
    amber: 'text-amber-500 dark:text-amber-400',
    rose: 'text-rose-500 dark:text-rose-400',
    blue: 'text-blue-500 dark:text-blue-400',
    violet: 'text-violet-500 dark:text-violet-400',
  };

  const iconBgs = {
    indigo: 'bg-indigo-50 dark:bg-indigo-950/40',
    emerald: 'bg-emerald-50 dark:bg-emerald-950/40',
    amber: 'bg-amber-50 dark:bg-amber-950/40',
    rose: 'bg-rose-50 dark:bg-rose-950/40',
    blue: 'bg-blue-50 dark:bg-blue-950/40',
    violet: 'bg-violet-50 dark:bg-violet-950/40',
  };
</script>

{#if href}
  <a
    {href}
    class="block p-4 rounded-2xl bg-white dark:bg-slate-800/80 border border-slate-100 dark:border-slate-700/60
           transition-all duration-200 hover:shadow-md hover:-translate-y-0.5 cursor-pointer group"
  >
    <div class="flex items-center justify-between mb-2">
      <span class="text-xs font-medium text-slate-400 dark:text-slate-500">{title}</span>
      <div class="w-8 h-8 rounded-lg {iconBgs[color]} flex items-center justify-center {iconColors[color]} text-base">
        {icon}
      </div>
    </div>
    <p class="text-2xl font-semibold text-slate-800 dark:text-white tracking-tight">{value}</p>
  </a>
{:else}
  <div class="p-4 rounded-2xl bg-white dark:bg-slate-800/80 border border-slate-100 dark:border-slate-700/60">
    <div class="flex items-center justify-between mb-2">
      <span class="text-xs font-medium text-slate-400 dark:text-slate-500">{title}</span>
      <div class="w-8 h-8 rounded-lg {iconBgs[color]} flex items-center justify-center {iconColors[color]} text-base">
        {icon}
      </div>
    </div>
    <p class="text-2xl font-semibold text-slate-800 dark:text-white tracking-tight">{value}</p>
  </div>
{/if}
