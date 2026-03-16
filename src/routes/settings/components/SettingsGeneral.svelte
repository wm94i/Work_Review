<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { enable as enableAutostart, disable as disableAutostart, isEnabled as isAutostartEnabled } from '@tauri-apps/plugin-autostart';

  export let config;

  const dispatch = createEventDispatcher();

  // 开机自启动状态（独立于 config，由系统 API 驱动）
  let autoStartEnabled = false;

  onMount(async () => {
    try {
      autoStartEnabled = await isAutostartEnabled();
      if (config.auto_start !== autoStartEnabled) {
        config.auto_start = autoStartEnabled;
        dispatch('change', config);
      }
    } catch (e) {
      console.error('查询自启动状态失败:', e);
    }
  });

  // 小时选项 (0-23)
  const hours = Array.from({ length: 24 }, (_, i) => i);
  // 分钟选项 (0, 15, 30, 45)
  const minutes = [0, 15, 30, 45];

  // 解析工作时间
  $: startHour = config.work_start_hour ?? 9;
  $: startMinute = config.work_start_minute ?? 0;
  $: endHour = config.work_end_hour ?? 18;
  $: endMinute = config.work_end_minute ?? 0;

  // 格式化为 HH:MM
  $: startTimeDisplay = `${String(startHour).padStart(2, '0')}:${String(startMinute).padStart(2, '0')}`;
  $: endTimeDisplay = `${String(endHour).padStart(2, '0')}:${String(endMinute).padStart(2, '0')}`;

  // 工作时长
  $: workHours = (() => {
    const startTotal = startHour * 60 + startMinute;
    const endTotal = endHour * 60 + endMinute;
    const diff = endTotal - startTotal;
    if (diff <= 0) return '—';
    const h = Math.floor(diff / 60);
    const m = diff % 60;
    return m > 0 ? `${h}小时${m}分钟` : `${h}小时`;
  })();

  function updateStart(h, m) {
    config.work_start_hour = h;
    config.work_start_minute = m;
    dispatch('change', config);
  }

  function updateEnd(h, m) {
    config.work_end_hour = h;
    config.work_end_minute = m;
    dispatch('change', config);
  }

  function handleChange() {
    dispatch('change', config);
  }

  // 开机自启动切换
  async function toggleAutoStart() {
    try {
      if (autoStartEnabled) {
        await disableAutostart();
        autoStartEnabled = false;
      } else {
        await enableAutostart();
        autoStartEnabled = true;
      }
      config.auto_start = autoStartEnabled;
      dispatch('change', config);
    } catch (e) {
      console.error('设置开机自启动失败:', e);
    }
  }

  // Dock 图标
  async function toggleDockIcon() {
    config.hide_dock_icon = !config.hide_dock_icon;
    try {
      await invoke('set_dock_visibility', { visible: !config.hide_dock_icon });
    } catch (e) {
      console.error('设置 Dock 图标失败:', e);
    }
    dispatch('change', config);
  }
</script>

<!-- 基本设置 -->
<div class="card p-5">
  <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-200 mb-1">基本设置</h3>
  <p class="text-xs text-slate-400 dark:text-slate-500 mb-4">工作时间和应用行为</p>

  <div class="space-y-4">
    <!-- 工作时间 -->
    <div>
      <div class="flex items-center justify-between mb-3">
        <span class="text-sm font-medium text-slate-700 dark:text-slate-300">工作时间</span>
        <span class="text-xs text-slate-400">共 {workHours}</span>
      </div>

      <div class="flex items-center gap-3">
        <!-- 开始时间 -->
        <div class="flex items-center gap-1.5 bg-slate-50 dark:bg-slate-700/50 rounded-lg px-3 py-2">
          <span class="text-xs text-slate-400">从</span>
          <input
            type="time"
            value={startTimeDisplay}
            on:change={(e) => {
              const [h, m] = e.target.value.split(':').map(Number);
              updateStart(h, m);
            }}
            class="bg-transparent text-sm font-mono text-slate-800 dark:text-white focus:outline-none"
          />
        </div>

        <span class="text-slate-300 dark:text-slate-600">—</span>

        <!-- 结束时间 -->
        <div class="flex items-center gap-1.5 bg-slate-50 dark:bg-slate-700/50 rounded-lg px-3 py-2">
          <span class="text-xs text-slate-400">到</span>
          <input
            type="time"
            value={endTimeDisplay}
            on:change={(e) => {
              const [h, m] = e.target.value.split(':').map(Number);
              updateEnd(h, m);
            }}
            class="bg-transparent text-sm font-mono text-slate-800 dark:text-white focus:outline-none"
          />
        </div>
      </div>
      <p class="text-xs text-slate-400 mt-2">此时间段内的活动将被计入工作时长统计</p>
    </div>

    <hr class="border-slate-200 dark:border-slate-700" />

    <!-- 开机自启动 -->
    <div class="flex items-center justify-between">
      <div>
        <div class="text-sm font-medium text-slate-700 dark:text-slate-300">开机自启动</div>
        <div class="text-xs text-slate-400 mt-0.5">系统启动时自动运行 Work Review</div>
      </div>
      <button
        on:click={toggleAutoStart}
        class="relative w-11 h-6 rounded-full transition-colors duration-200 {autoStartEnabled ? 'bg-primary-500' : 'bg-slate-300 dark:bg-slate-600'}"
      >
        <span class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 {autoStartEnabled ? 'translate-x-5' : 'translate-x-0'}"></span>
      </button>
    </div>

    <hr class="border-slate-200 dark:border-slate-700" />

    <!-- Dock 图标 -->
    <div class="flex items-center justify-between">
      <div>
        <div class="text-sm font-medium text-slate-700 dark:text-slate-300">隐藏 Dock 图标</div>
        <div class="text-xs text-slate-400 mt-0.5">隐藏后仅通过系统托盘访问应用</div>
      </div>
      <button
        on:click={toggleDockIcon}
        class="relative w-11 h-6 rounded-full transition-colors duration-200 {config.hide_dock_icon ? 'bg-primary-500' : 'bg-slate-300 dark:bg-slate-600'}"
      >
        <span class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 {config.hide_dock_icon ? 'translate-x-5' : 'translate-x-0'}"></span>
      </button>
    </div>
  </div>
</div>
